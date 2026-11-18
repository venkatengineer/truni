use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruniConfig {
    pub ai: AiConfig,
    pub voice: VoiceConfig,
    pub overlay: OverlayConfig,
    pub telemetry: TelemetryConfig,
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub fallback_model: Option<String>,
    pub temperature: f32,
    pub context_limit: usize,
    pub unload_after_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub wake_word: String,
    pub sensitivity: f32,
    pub push_to_talk: bool,
    pub whisper_model_path: PathBuf,
    pub piper_model_path: PathBuf,
    pub piper_config_path: PathBuf,
    pub audio_device: Option<String>,
    pub silence_threshold_secs: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayConfig {
    pub width: i32,
    pub height: i32,
    pub keyboard_interactivity: bool,
    pub spaceship_render_fps: u32,
    pub spaceship_idle_fps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub poll_interval_ms: u64,
    pub idle_poll_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub db_path: PathBuf,
    pub allowed_directories: Vec<PathBuf>,
    pub require_approval_for_deletion: bool,
}

impl Default for TruniConfig {
    fn default() -> Self {
        let base_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/home/venkatengineer"))
            .join(".local/share/truni");

        Self {
            ai: AiConfig {
                provider: "ollama".to_string(),
                base_url: "http://127.0.0.1:11434".to_string(),
                model: "qwen2.5:1.5b".to_string(),
                fallback_model: Some("qwen2.5:1.5b".to_string()),
                temperature: 0.2,
                context_limit: 4096,
                unload_after_secs: 300,
            },
            voice: VoiceConfig {
                wake_word: "TRUNI".to_string(),
                sensitivity: 0.5,
                push_to_talk: true,
                whisper_model_path: PathBuf::from("/mnt/d/truni/models/whisper/ggml-tiny.en.bin"),
                piper_model_path: PathBuf::from("/mnt/d/truni/models/piper/en_US-lessac-medium.onnx"),
                piper_config_path: PathBuf::from("/mnt/d/truni/models/piper/en_US-lessac-medium.onnx.json"),
                audio_device: None,
                silence_threshold_secs: 1.5,
            },
            overlay: OverlayConfig {
                width: 960,
                height: 640,
                keyboard_interactivity: true,
                spaceship_render_fps: 60,
                spaceship_idle_fps: 15,
            },
            telemetry: TelemetryConfig {
                poll_interval_ms: 1000,
                idle_poll_interval_ms: 3000,
            },
            storage: StorageConfig {
                db_path: base_dir.join("truni_memory.sqlite"),
                allowed_directories: vec![
                    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/venkatengineer")),
                    PathBuf::from("/mnt/d"),
                ],
                require_approval_for_deletion: true,
            },
        }
    }
}
