use super::Encoder;

pub struct PerlEncoder;

impl Encoder for PerlEncoder {
    fn name(&self) -> &str {
        "perl"
    }

    fn encode_chunk(&self, data: &[u8], remote_path: &str, first: bool) -> String {
        let hex: String = data.iter().map(|b| format!("{:02x}", b)).collect();
        let redirect = if first { ">" } else { ">>" };
        format!("perl -e 'print pack(\"H*\",\"{}\")' {} {}", hex, redirect, remote_path)
    }

    fn verify_cmd(&self, remote_path: &str, _expected_hex: &str) -> String {
        format!("sha256sum {}", remote_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perl_encode_chunk_produces_valid_shell() {
        let data = [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0xFF];
        let enc = PerlEncoder;
        let tmp = std::env::temp_dir().join("ordo_test_perl");
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

    #[test]
    fn test_perl_encode_chunk_append() {
        let data1 = [0xDE, 0xAD];
        let data2 = [0xBE, 0xEF];
        let enc = PerlEncoder;
        let tmp = std::env::temp_dir().join("ordo_test_perl_append");
        let path = tmp.to_string_lossy().to_string();

        let cmd1 = enc.encode_chunk(&data1, &path, true);
        let cmd2 = enc.encode_chunk(&data2, &path, false);
        std::process::Command::new("sh").arg("-c").arg(&cmd1).status().unwrap();
        std::process::Command::new("sh").arg("-c").arg(&cmd2).status().unwrap();

        let result = std::fs::read(&path).expect("read failed");
        assert_eq!(result, [0xDE, 0xAD, 0xBE, 0xEF]);
        let _ = std::fs::remove_file(&path);
    }
}
