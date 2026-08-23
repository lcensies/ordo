use super::Source;
use std::io::Read;

pub struct StdinSource;

impl Source for StdinSource {
    fn name(&self) -> &str {
        "<stdin>"
    }

    fn read_chunk(&mut self, size: usize) -> anyhow::Result<Option<Vec<u8>>> {
        let mut buf = vec![0u8; size];
        let mut filled = 0;
        let mut stdin = std::io::stdin().lock();
        while filled < size {
            match stdin.read(&mut buf[filled..])? {
                0 => break,
                n => filled += n,
            }
        }
        if filled == 0 {
            return Ok(None);
        }
        buf.truncate(filled);
        Ok(Some(buf))
    }
}
