use super::OutputBackend;
use std::process::Command;

pub struct TmuxBackend {
    pub target: String,
}

impl OutputBackend for TmuxBackend {
    fn send_line(&mut self, cmd: &str) -> anyhow::Result<()> {
        let status = Command::new("tmux")
            .args(["send-keys", "-t", &self.target, cmd, "Enter"])
            .status()?;
        if !status.success() {
            anyhow::bail!("tmux send-keys failed with status {}", status);
        }
        Ok(())
    }

    fn capture_output(&mut self) -> anyhow::Result<Option<String>> {
        let output = Command::new("tmux")
            .args(["capture-pane", "-t", &self.target, "-p"])
            .output()?;
        if output.status.success() {
            Ok(Some(String::from_utf8_lossy(&output.stdout).to_string()))
        } else {
            Ok(None)
        }
    }
}
