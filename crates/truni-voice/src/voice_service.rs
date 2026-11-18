use crate::piper::PiperSynthesizer;
use crate::vad::VadDetector;
use crate::wake::WakeWordDetector;
use crate::whisper::WhisperTranscriber;
use anyhow::{bail, Result};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;
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

    pub async fn record_and_transcribe(&self, max_duration_secs: u64) -> Result<String> {
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

        // Use pw-record (PipeWire) or arecord (ALSA) to capture 16kHz mono WAV
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
                    wav_path.to_str().unwrap(),
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
                    "wav",
                    wav_path.to_str().unwrap(),
                ],
            )
        };

        let child = Command::new(rec_cmd)
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        let child = match child {
            Ok(c) => c,
            Err(e) => {
                let mut listening_lock = self.is_listening.lock().unwrap();
                *listening_lock = false;
                bail!("Failed to start recording tool {}: {}", rec_cmd, e);
            }
        };

        {
            let mut rec_lock = self.recording_process.lock().unwrap();
            *rec_lock = Some(child);
        }

        // Wait for max_duration_secs while recording
        sleep(Duration::from_secs(max_duration_secs)).await;

        // Cleanly stop recording with SIGINT so WAV header is properly closed
        {
            let mut rec_lock = self.recording_process.lock().unwrap();
            if let Some(mut child) = rec_lock.take() {
                unsafe {
                    libc::kill(child.id() as i32, libc::SIGINT);
                }
                let _ = child.wait();
            }
        }

        {
            let mut listening_lock = self.is_listening.lock().unwrap();
            *listening_lock = false;
        }

        // Verify audio file exists and has data
        if !wav_path.exists() {
            bail!("Audio capture failed: no output file generated at {:?}", wav_path);
        }

        let metadata = std::fs::metadata(&wav_path)?;
        if metadata.len() < 1024 {
            // Empty or virtually silent audio
            return Ok(String::new());
        }

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
