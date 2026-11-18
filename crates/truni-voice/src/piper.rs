use anyhow::{bail, Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

pub struct PiperSynthesizer {
    model_path: PathBuf,
    config_path: PathBuf,
    current_playback: Arc<Mutex<Option<Child>>>,
}

impl PiperSynthesizer {
    pub fn new<P: AsRef<Path>>(model_path: P, config_path: P) -> Self {
        Self {
            model_path: model_path.as_ref().to_path_buf(),
            config_path: config_path.as_ref().to_path_buf(),
            current_playback: Arc::new(Mutex::new(None)),
        }
    }

    pub fn stop_speaking(&self) {
        let mut lock = self.current_playback.lock().unwrap();
        if let Some(mut child) = lock.take() {
            let _ = child.kill();
        }
    }

    pub fn is_speaking(&self) -> bool {
        let mut lock = self.current_playback.lock().unwrap();
        if let Some(child) = lock.as_mut() {
            match child.try_wait() {
                Ok(None) => true,
                _ => {
                    *lock = None;
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn speak(&self, text: &str) -> Result<()> {
        self.stop_speaking();

        if !self.model_path.exists() {
            bail!("Piper model not found at {:?}", self.model_path);
        }

        // Ensure PipeWire audio sink is unmuted
        let _ = Command::new("wpctl")
            .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "0"])
            .output();

        let wav_path = std::env::temp_dir().join("truni_tts_playback.wav");

        let mut piper_child = Command::new("piper")
            .arg("-m")
            .arg(&self.model_path)
            .arg("-c")
            .arg(&self.config_path)
            .arg("-f")
            .arg(&wav_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| "Failed to spawn piper")?;

        if let Some(mut stdin) = piper_child.stdin.take() {
            stdin.write_all(text.as_bytes())?;
        }

        let status = piper_child.wait()?;
        if !status.success() {
            bail!("Piper synthesis exited with error: {:?}", status);
        }

        // Play audio with pw-play or aplay
        let play_cmd = if which_cmd("pw-play") {
            "pw-play"
        } else {
            "aplay"
        };

        let play_child = Command::new(play_cmd)
            .arg(&wav_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("Failed to spawn {}", play_cmd))?;

        let mut lock = self.current_playback.lock().unwrap();
        *lock = Some(play_child);

        Ok(())
    }

    pub fn speak_blocking(&self, text: &str) -> Result<()> {
        self.speak(text)?;
        loop {
            if !self.is_speaking() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        Ok(())
    }
}

fn which_cmd(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
