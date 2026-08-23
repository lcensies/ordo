use super::Source;
use std::fs::{self, File};
use std::io::Read;

pub struct FileSource {
    path: String,
    file: Option<File>,
}

impl FileSource {
    pub fn new(path: &str) -> Self {
        FileSource {
            path: path.to_string(),
            file: None,
        }
    }
}

impl Source for FileSource {
    fn name(&self) -> &str {
        &self.path
    }

    fn size_hint(&self) -> Option<u64> {
        fs::metadata(&self.path).ok().map(|m| m.len())
    }

    fn read_chunk(&mut self, size: usize) -> anyhow::Result<Option<Vec<u8>>> {
        if self.file.is_none() {
            self.file = Some(File::open(&self.path)?);
        }
        let f = self.file.as_mut().unwrap();
        let mut buf = vec![0u8; size];
        let mut filled = 0;
        while filled < size {
            match f.read(&mut buf[filled..])? {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_source_streams_in_chunks() {
        let data: Vec<u8> = (0..=255u8).cycle().take(1000).collect();
        let tmp = std::env::temp_dir().join("ordo_test_filesource");
        std::fs::write(&tmp, &data).unwrap();

        let mut src = FileSource::new(tmp.to_str().unwrap());
        assert_eq!(src.size_hint(), Some(1000));
        let mut got = Vec::new();
        while let Some(chunk) = src.read_chunk(300).unwrap() {
            assert!(chunk.len() <= 300);
            got.extend_from_slice(&chunk);
        }
        assert_eq!(got, data);
        let _ = std::fs::remove_file(&tmp);
    }
}
