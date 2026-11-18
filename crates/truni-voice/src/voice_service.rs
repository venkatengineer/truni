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
    config: VoiceConfig,
    whisper: Arc<WhisperTranscriber>,
    piper: Arc<PiperSynthesizer>,
    wake_detector: Arc<Mutex<WakeWordDetector>>,
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

    pub fn acknowledge_wake(&self) -> Result<()> {
        self.speak("Systems online. How can I help?")
    }

    pub async fn record_and_transcribe(&self, max_duration_secs: u64) -> Result<String> {
        let temp_wav = tempfile::Builder::new()
            .prefix("truni_rec_")
            .suffix(".wav")
            .tempfile()?;

        let wav_path = temp_wav.path().to_path_buf();

        {
            let mut listening_lock = self.is_listening.lock().unwrap();
            *listening_lock = true;
        }

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

        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                let mut listening_lock = self.is_listening.lock().unwrap();
                *listening_lock = false;
                bail!("Failed to start recording tool {}: {}", rec_cmd, e);
            }
        };

        {
            let mut rec_lock = self.recording_process.lock().unwrap();
            *rec_lock = None;
        }

        // Wait for max_duration_secs or until user stops speaking
        sleep(Duration::from_secs(max_duration_secs)).await;

        let _ = child.kill();
        let _ = child.wait();

        {
            let mut listening_lock = self.is_listening.lock().unwrap();
            *listening_lock = false;
        }

        // Transcribe with Whisper
        let transcription = self.whisper.transcribe(&wav_path)?;
        Ok(transcription)
    }

    pub fn cancel_recording(&self) {
        let mut rec_lock = self.recording_process.lock().unwrap();
        if let Some(mut child) = rec_lock.take() {
            let _ = child.kill();
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
