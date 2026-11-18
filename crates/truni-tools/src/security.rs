use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Safe,
    RequiresApproval,
    Forbidden,
}

pub struct SecurityManager {
    destructive_commands: Vec<&'static str>,
    protected_paths: Vec<&'static str>,
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityManager {
    pub fn new() -> Self {
        Self {
            destructive_commands: vec![
                "rm ", "rmdir", "mkfs", "dd ", "fdisk", "parted",
                "reboot", "shutdown", "poweroff", "init 0",
                "chmod -R 777", "chown -R", ":(){ :|:& };:",
                "pacman -R", "pacman -S", "pip install",
            ],
            protected_paths: vec![
                "/etc/shadow", "/etc/sudoers", "/boot", "/dev",
                "/sys", "/proc", "/root",
            ],
        }
    }

    pub fn assess_command(&self, command: &str) -> RiskLevel {
        let trimmed = command.trim();

        // Check for forbidden commands or root tampering
        if trimmed.starts_with("sudo rm -rf /") || trimmed.contains(":(){ :|:& };:") {
            return RiskLevel::Forbidden;
        }

        // Check for destructive patterns
        for bad in &self.destructive_commands {
            if trimmed.contains(bad) {
                return RiskLevel::RequiresApproval;
            }
        }

        RiskLevel::Safe
    }

    pub fn assess_file_write(&self, path: &Path, is_delete: bool) -> RiskLevel {
        let path_str = path.to_string_lossy();

        for protected in &self.protected_paths {
            if path_str.starts_with(protected) {
                return RiskLevel::Forbidden;
            }
        }

        if is_delete {
            return RiskLevel::RequiresApproval;
        }

        RiskLevel::Safe
    }
}
