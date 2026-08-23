pub mod chmod;
pub mod exec;
pub mod ssh;
pub mod stdout;
pub mod tmux;

pub trait OutputBackend {
    fn send_line(&mut self, cmd: &str) -> anyhow::Result<()>;
    fn capture_output(&mut self) -> anyhow::Result<Option<String>>;
}
