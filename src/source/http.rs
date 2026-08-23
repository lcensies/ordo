use super::file::FileSource;
use super::Source;
use anyhow::Context;
use std::fs::{self, File};
use std::path::PathBuf;

/// Streams the URL to a temp file, then serves chunked reads from disk.
/// No curl dependency (pure-Rust ureq); constant memory regardless of size.
pub struct HttpSource {
    url: String,
    tmp: Option<PathBuf>,
    inner: Option<FileSource>,
}

impl HttpSource {
    pub fn new(url: &str) -> Self {
        HttpSource {
            url: url.to_string(),
            tmp: None,
            inner: None,
        }
    }

    fn ensure_downloaded(&mut self) -> anyhow::Result<()> {
        if self.inner.is_some() {
            return Ok(());
        }
        eprintln!("downloading {} locally ...", self.url);
        let mut resp = ureq::get(&self.url)
            .call()
            .with_context(|| format!("GET {} failed", self.url))?;
        let tmp = std::env::temp_dir().join(format!("ordo-dl-{}", std::process::id()));
        {
            let mut f = File::create(&tmp)?;
            let mut reader = resp.body_mut().as_reader();
            let n = std::io::copy(&mut reader, &mut f)?;
            eprintln!("downloaded {} bytes -> {}", n, tmp.display());
        }
        self.inner = Some(FileSource::new(tmp.to_str().context("bad tmp path")?));
        self.tmp = Some(tmp);
        Ok(())
    }
}

impl Source for HttpSource {
    fn name(&self) -> &str {
        &self.url
    }

    fn size_hint(&self) -> Option<u64> {
        self.tmp
            .as_ref()
            .and_then(|t| fs::metadata(t).ok())
            .map(|m| m.len())
    }

    fn read_chunk(&mut self, size: usize) -> anyhow::Result<Option<Vec<u8>>> {
        self.ensure_downloaded()?;
        self.inner.as_mut().unwrap().read_chunk(size)
    }
}

impl Drop for HttpSource {
    fn drop(&mut self) {
        if let Some(tmp) = &self.tmp {
            let _ = fs::remove_file(tmp);
        }
    }
}
