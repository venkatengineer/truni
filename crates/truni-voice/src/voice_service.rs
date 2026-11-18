use crate::piper::PiperSynthesizer;
use crate::vad::VadDetector;
use crate::wake::WakeWordDetector;
use crate::whisper::WhisperTranscriber;
use anyhow::{bail, Result};
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use truni_core::config::VoiceConfig;

#[derive(Clone)]
pub struct VoiceService {
    #[allow(dead_code)]
    config: VoiceConfig,
    whisper: Arc<WhisperTranscriber>,
    piper: Arc<PiperSynthesizer>,
    #[allow(dead_code)]
    wake_detector: Arc<Mutex<WakeWordDetector>>,
    #[allow(dead_code)]
    vad: Arc<Mutex<VadDetector>>,
    recording_process: Arc<Mutex<Option<Child>>>,
    is_listening: Arc<Mutex<bool>>,
}

impl VoiceService {
    pub fn new(config: &VoiceConfig) -> Self {
        let whisper = Arc::new(WhisperTranscriber::new(&config.whisper_model_path));
        let piper = Arc::new(PiperSynthesizer::new(
            &config.piper_model_path,
            &config.piper_config_path,
        ));
        let wake_detector = Arc::new(Mutex::new(WakeWordDetector::new(&config.wake_word)));
        let vad = Arc::new(Mutex::new(VadDetector::default()));

        Self {
            config: config.clone(),
            whisper,
            piper,
            wake_detector,
            vad,
            recording_process: Arc::new(Mutex::new(None)),
            is_listening: Arc::new(Mutex::new(false)),
        }
    }

    pub fn is_listening(&self) -> bool {
        *self.is_listening.lock().unwrap()
    }

    pub fn is_speaking(&self) -> bool {
        self.piper.is_speaking()
    }

    pub fn stop_speaking(&self) {
        self.piper.stop_speaking();
    }

    pub fn speak(&self, text: &str) -> Result<()> {
        self.piper.speak(text)
    }

    pub fn speak_blocking(&self, text: &str) -> Result<()> {
        self.piper.speak_blocking(text)
    }

    pub fn acknowledge_wake(&self) -> Result<()> {
        self.speak("Systems online. How can I help?")
    }

    pub async fn record_and_transcribe(&self, _max_duration_secs: u64) -> Result<String> {
        let wav_path = std::env::temp_dir().join("truni_mic_capture.wav");
        if wav_path.exists() {
            let _ = std::fs::remove_file(&wav_path);
        }

        {
            let mut listening_lock = self.is_listening.lock().unwrap();
            *listening_lock = true;
        }

        // Ensure PipeWire microphone source is unmuted and at optimal gain
        let _ = Command::new("wpctl")
            .args(["set-mute", "@DEFAULT_AUDIO_SOURCE@", "0"])
            .output();
        let _ = Command::new("wpctl")
            .args(["set-volume", "@DEFAULT_AUDIO_SOURCE@", "1.0"])
            .output();

        // Use pw-record (PipeWire) or arecord (ALSA) to stream 16kHz mono PCM directly to stdout
        let (rec_cmd, args) = if which_cmd("pw-record") {
            (
                "pw-record",
                vec![
                    "--rate",
                    "16000",
                    "--channels",
                    "1",
                    "--format",
                    "s16",
                    "-",
                ],
            )
        } else {
            (
                "arecord",
                vec![
                    "-r",
                    "16000",
                    "-c",
                    "1",
                    "-f",
                    "S16_LE",
                    "-t",
                    "raw",
                    "-",
                ],
            )
        };

        let child = Command::new(rec_cmd)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();

        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                let mut listening_lock = self.is_listening.lock().unwrap();
                *listening_lock = false;
                bail!("Failed to start recording tool {}: {}", rec_cmd, e);
            }
        };

        let stdout = child.stdout.take().expect("Child stdout available");

        // Run dynamic VAD capture in a blocking thread so it doesn't block the async runtime
        let capture_result = tokio::task::spawn_blocking(move || {
            let mut reader = stdout;
            let mut pcm_buffer = Vec::with_capacity(16000 * 2 * 10);
            let mut frame = [0u8; 2048]; // 1024 samples = 64ms at 16kHz
            let mut speech_detected = false;
            let mut silence_frames = 0;
            let mut initial_silence_frames = 0;

            loop {
                let n = match reader.read(&mut frame) {
                    Ok(0) => break,
                    Ok(bytes_read) => bytes_read,
                    Err(_) => break,
                };

                pcm_buffer.extend_from_slice(&frame[..n]);

                let samples: Vec<i16> = frame[..n]
                    .chunks_exact(2)
                    .map(|c| i16::from_le_bytes([c[0], c[1]]))
                    .collect();

                if !samples.is_empty() {
                    let sum_sq: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
                    let rms = (sum_sq / (samples.len() as f64)).sqrt() / 32768.0;

                    if rms > 0.012 {
                        speech_detected = true;
                        silence_frames = 0;
                    } else if speech_detected {
                        silence_frames += 1;
                        // User has stopped speaking: wait for 22 frames (~1.4 seconds of sustained silence)
                        if silence_frames >= 22 {
                            break;
                        }
                    } else {
                        initial_silence_frames += 1;
                        // If user doesn't start speaking within ~5.8s, stop
                        if initial_silence_frames >= 90 {
                            break;
                        }
                    }
                }

                // Generous safety timeout: 25 seconds of speech
                if pcm_buffer.len() >= 16000 * 2 * 25 {
                    break;
                }
            }

            // Cleanly terminate recording process with SIGINT
            unsafe {
                libc::kill(child.id() as i32, libc::SIGINT);
            }
            let _ = child.wait();

            (speech_detected, pcm_buffer)
        })
        .await?;

        {
            let mut listening_lock = self.is_listening.lock().unwrap();
            *listening_lock = false;
        }

        let (speech_detected, pcm_data) = capture_result;

        if !speech_detected || pcm_data.len() < 16000 {
            // No speech detected
            return Ok(String::new());
        }

        // Write valid RIFF WAV with standard 44-byte header
        let wav_bytes = encode_wav(&pcm_data, 16000);
        std::fs::write(&wav_path, &wav_bytes)?;

        // Transcribe with Whisper
        let transcription = self.whisper.transcribe(&wav_path)?;
        Ok(transcription)
    }

    pub fn cancel_recording(&self) {
        let mut rec_lock = self.recording_process.lock().unwrap();
        if let Some(mut child) = rec_lock.take() {
            unsafe {
                libc::kill(child.id() as i32, libc::SIGINT);
            }
            let _ = child.wait();
        }
        let mut listening_lock = self.is_listening.lock().unwrap();
        *listening_lock = false;
    }
}

fn which_cmd(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn encode_wav(pcm_data: &[u8], sample_rate: u32) -> Vec<u8> {
    let mut wav = Vec::with_capacity(44 + pcm_data.len());
    let data_len = pcm_data.len() as u32;
    let riff_len = data_len + 36;
    let byte_rate = sample_rate * 2;

    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&riff_len.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(pcm_data);

    wav
}
