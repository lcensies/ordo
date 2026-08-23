use crate::backend::OutputBackend;
use crate::encoder::Encoder;
use crate::source::Source;
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use std::thread;
use std::time::Duration;

#[allow(clippy::too_many_arguments)]
pub fn run(
    source: &mut dyn Source,
    encoder: &dyn Encoder,
    backend: &mut dyn OutputBackend,
    remote_path: &str,
    chunk_size: usize,
    verify: bool,
    delay_ms: u64,
    chmod: Option<&str>,
    chmod_method: crate::backend::chmod::ChmodMethod,
    exec: Option<&str>,
) -> anyhow::Result<()> {
    // Stream chunks; hash incrementally so memory stays O(chunk_size)
    let mut hasher = Sha256::new();
    let total = source.size_hint();
    let pb = match total {
        Some(t) => ProgressBar::new(t),
        None => ProgressBar::new_spinner(),
    };
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes} ({bytes_per_sec})")?
            .progress_chars("#>-"),
    );

    let mut first = true;
    while let Some(chunk) = source.read_chunk(chunk_size)? {
        hasher.update(&chunk);
        let cmd = encoder.encode_chunk(&chunk, remote_path, first);
        first = false;
        backend.send_line(&cmd)?;
        pb.inc(chunk.len() as u64);
        if delay_ms > 0 {
            thread::sleep(Duration::from_millis(delay_ms));
        }
    }
    let expected_hex = hex::encode(hasher.finalize());

    pb.finish_with_message(format!("transfer complete: {}", source.name()));

    if let Some(mode) = chmod {
        backend.send_line(&chmod_method.cmd(mode, remote_path))?;
        eprintln!("chmod 0{} {}", mode.trim_start_matches('0'), remote_path);
    }

    if verify {
        let verify_cmd = encoder.verify_cmd(remote_path, &expected_hex);
        backend.send_line(&verify_cmd)?;
        if let Some(output) = backend.capture_output()? {
            if output.contains(&expected_hex) {
                eprintln!("Checksum verified: {}", expected_hex);
            } else {
                anyhow::bail!(
                    "Checksum mismatch! Expected {} but got: {}",
                    expected_hex,
                    output.trim()
                );
            }
        } else {
            eprintln!("Verification command sent (no output capture available for this backend)");
        }
    }

    // exec last: tool output must not desync verify's stdout capture
    if let Some(args) = exec {
        let cmd = format!("\"{}\" {}", remote_path, args).trim().to_string();
        backend.send_line(&cmd)?;
        eprintln!("exec: {}", cmd);
    }

    Ok(())
}
