use super::OutputBackend;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

pub struct SshBackend {
    pub spec: String,
    child: Option<Child>,
    stdout_reader: Option<BufReader<std::process::ChildStdout>>,
}

impl SshBackend {
    pub fn new(spec: &str) -> Self {
        SshBackend {
            spec: spec.to_string(),
            child: None,
            stdout_reader: None,
        }
    }

    fn ensure_child(&mut self) -> anyhow::Result<()> {
        if self.child.is_none() {
            let mut child = Command::new("ssh")
                .args(["-T", &self.spec, "sh"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()?;
            let stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("no stdout"))?;
            self.stdout_reader = Some(BufReader::new(stdout));
            self.child = Some(child);
        }
        Ok(())
    }
}

impl OutputBackend for SshBackend {
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
        if let Some(reader) = &mut self.stdout_reader {
            let mut line = String::new();
            reader.read_line(&mut line)?;
            if line.is_empty() {
                Ok(None)
            } else {
                Ok(Some(line))
            }
        } else {
            Ok(None)
        }
    }
}

impl Drop for SshBackend {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            // Close stdin to let remote sh exit
            child.stdin.take();
            let _ = child.wait();
        }
    }
}
