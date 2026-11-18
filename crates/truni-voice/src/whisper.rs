use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct WhisperTranscriber {
    model_path: PathBuf,
}

impl WhisperTranscriber {
    pub fn new<P: AsRef<Path>>(model_path: P) -> Self {
        Self {
            model_path: model_path.as_ref().to_path_buf(),
        }
    }

    pub fn transcribe<P: AsRef<Path>>(&self, wav_path: P) -> Result<String> {
        if !self.model_path.exists() {
            bail!("Whisper model not found at {:?}", self.model_path);
        }

        let output = Command::new("whisper-cli")
            .arg("-m")
            .arg(&self.model_path)
            .arg("-f")
            .arg(wav_path.as_ref())
            .arg("-nt") // no timestamps
            .output()
            .with_context(|| "Failed to execute whisper-cli")?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            bail!("Whisper transcription failed: {}", err);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Clean out header lines or system_info lines
        let mut clean_text = String::new();
        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty()
                || trimmed.starts_with("system_info")
                || trimmed.starts_with("main:")
                || trimmed.starts_with("whisper_")
                || trimmed.starts_with("load_backend")
                || trimmed.starts_with("read_audio_data")
            {
                continue;
            }
            clean_text.push_str(trimmed);
            clean_text.push(' ');
        }

        Ok(clean_text.trim().to_string())
    }
}
