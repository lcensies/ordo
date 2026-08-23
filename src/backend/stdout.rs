use super::OutputBackend;

pub struct StdoutBackend;

impl OutputBackend for StdoutBackend {
    fn send_line(&mut self, cmd: &str) -> anyhow::Result<()> {
        println!("{}", cmd);
        Ok(())
    }

    fn capture_output(&mut self) -> anyhow::Result<Option<String>> {
        Ok(None)
    }
}
