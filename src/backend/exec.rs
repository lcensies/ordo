use super::OutputBackend;
use std::io::Write;
use std::process::{Child, Command, Stdio};

pub struct ExecBackend {
    pub cmd: String,
    child: Option<Child>,
}

impl ExecBackend {
    pub fn new(cmd: &str) -> Self {
        ExecBackend {
            cmd: cmd.to_string(),
            child: None,
        }
    }

    fn ensure_child(&mut self) -> anyhow::Result<()> {
        if self.child.is_none() {
            let child = Command::new("sh")
                .arg("-c")
                .arg(&self.cmd)
                .stdin(Stdio::piped())
                .spawn()?;
            self.child = Some(child);
        }
        Ok(())
    }
}

impl OutputBackend for ExecBackend {
    fn send_line(&mut self, cmd: &str) -> anyhow::Result<()> {
        self.ensure_child()?;
        if let Some(child) = &mut self.child {
            if let Some(stdin) = &mut child.stdin {
                stdin.write_all(cmd.as_bytes())?;
                stdin.write_all(b"\n")?;
            }
        }
        Ok(())
    }

    fn capture_output(&mut self) -> anyhow::Result<Option<String>> {
        Ok(None)
    }
}
