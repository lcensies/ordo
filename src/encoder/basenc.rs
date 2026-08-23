use super::Encoder;

/// base32 (RFC 4648) via coreutils `basenc --base32` — less signatured than
/// the base64 tool every EDR rules on. No crate: 10 lines of bit shifting.
pub struct BasencEncoder;

fn b32_encode(data: &[u8]) -> String {
    const A: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut out = String::new();
    let mut acc = 0u64;
    let mut bits = 0u32;
    for &b in data {
        acc = (acc << 8) | b as u64;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(A[((acc >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(A[((acc << (5 - bits)) & 31) as usize] as char);
    }
    while out.len() % 8 != 0 {
        out.push('=');
    }
    out
}

impl Encoder for BasencEncoder {
    fn name(&self) -> &str {
        "basenc"
    }

    fn encode_chunk(&self, data: &[u8], remote_path: &str, first: bool) -> String {
        let redirect = if first { ">" } else { ">>" };
        // printf, not echo: echo appends \n (needs -n, not portable)
        format!(
            "printf '%s' '{}' | basenc --base32 -d {} {}",
            b32_encode(data),
            redirect,
            remote_path
        )
    }

    fn verify_cmd(&self, remote_path: &str, _expected_hex: &str) -> String {
        format!("sha256sum {}", remote_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_b32_matches_coreutils() {
        // reference: basenc --base32 itself
        let data: Vec<u8> = (0..=255u8).cycle().take(37).collect();
        let mut child = match std::process::Command::new("basenc")
            .args(["--base32"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => return, // no basenc on this host — skip
        };
        use std::io::Write;
        child.stdin.take().unwrap().write_all(&data).unwrap();
        let out = child.wait_with_output().unwrap();
        let reference = String::from_utf8(out.stdout).unwrap();
        assert_eq!(b32_encode(&data), reference.trim_end());
    }

    #[test]
    fn test_basenc_encode_chunk_roundtrip() {
        let data = [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0xFF, 0x42];
        let enc = BasencEncoder;
        let tmp = std::env::temp_dir().join("ordo_test_basenc");
        let path = tmp.to_string_lossy().to_string();

        let cmd = enc.encode_chunk(&data, &path, true);
        let status = std::process::Command::new("sh").arg("-c").arg(&cmd).status();
        match status {
            Ok(s) if s.success() => {}
            _ => return, // no basenc — skip
        }
        let result = std::fs::read(&path).unwrap();
        assert_eq!(result, data);
        let _ = std::fs::remove_file(&path);
    }
}
