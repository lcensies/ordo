pub mod file;
pub mod http;
pub mod stdin;

pub trait Source {
    fn name(&self) -> &str;
    fn size_hint(&self) -> Option<u64> {
        None
    }
    /// Next chunk of up to `size` bytes; Ok(None) at EOF.
    fn read_chunk(&mut self, size: usize) -> anyhow::Result<Option<Vec<u8>>>;
}
