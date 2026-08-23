use super::Encoder;

pub struct PrintfEncoder;

impl Encoder for PrintfEncoder {
    fn name(&self) -> &str {
        "printf"
    }

    fn encode_chunk(&self, data: &[u8], remote_path: &str, first: bool) -> String {
        let escaped: String = data.iter().map(|b| format!("\\{:03o}", b)).collect();
        let redirect = if first { ">" } else { ">>" };
        format!("printf '{}' {} {}", escaped, redirect, remote_path)
    }

    fn verify_cmd(&self, remote_path: &str, _expected_hex: &str) -> String {
        format!("sha256sum {}", remote_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_printf_encode_chunk_produces_valid_shell() {
        let data = [0x41, 0x42, 0x43]; // ABC - avoid null bytes for printf test
        let enc = PrintfEncoder;
        let tmp = std::env::temp_dir().join("ordo_test_printf");
        let path = tmp.to_string_lossy().to_string();

        let cmd = enc.encode_chunk(&data, &path, true);
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .status()
            .expect("sh failed");
        assert!(status.success());

        let result = std::fs::read(&path).expect("read failed");
        assert_eq!(result, data);
        let _ = std::fs::remove_file(&path);
    }
}
