use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

pub struct WakeWordDetector {
    keyword: String,
    python_worker: Option<Child>,
}

impl WakeWordDetector {
    pub fn new(keyword: &str) -> Self {
        Self {
            keyword: keyword.to_string(),
            python_worker: None,
        }
    }

    pub fn start_worker(&mut self, script_path: PathBuf) {
        if script_path.exists() {
            if let Ok(child) = Command::new("python")
                .arg(script_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
            {
                self.python_worker = Some(child);
            }
        }
    }

    pub fn matches_text(&self, text: &str) -> bool {
        let t = text.to_lowercase();
        let k = self.keyword.to_lowercase();
        t.contains(&k) || t.contains("truni") || t.contains("trooni") || t.contains("truney")
    }
}
