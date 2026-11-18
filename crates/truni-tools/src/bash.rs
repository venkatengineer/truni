use anyhow::{bail, Result};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

pub struct BashTool;

pub struct CommandResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

impl BashTool {
    pub async fn execute(command: &str, timeout_secs: u64) -> Result<CommandResult> {
        let future = Command::new("bash")
            .arg("-c")
            .arg(command)
            .output();

        let output = match timeout(Duration::from_secs(timeout_secs), future).await {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => bail!("Command execution failed: {}", e),
            Err(_) => bail!("Command timed out after {} seconds", timeout_secs),
        };

        Ok(CommandResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            success: output.status.success(),
        })
    }

    pub async fn inspect_system_logs(lines: usize) -> Result<String> {
        let cmd = format!("journalctl -n {} --no-pager", lines);
        let res = Self::execute(&cmd, 5).await?;
        Ok(res.stdout)
    }
}
