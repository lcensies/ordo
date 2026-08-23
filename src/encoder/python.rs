use super::Encoder;
use base64::{engine::general_purpose::STANDARD, Engine};

pub struct PythonEncoder;

impl Encoder for PythonEncoder {
    fn name(&self) -> &str {
        "python"
    }

    fn encode_chunk(&self, data: &[u8], remote_path: &str, first: bool) -> String {
        let encoded = STANDARD.encode(data);
        let mode = if first { "wb" } else { "ab" };
        format!(
            "python3 -c \"import base64;f=open('{}','{}');f.write(base64.b64decode('{}'));f.close()\"",
            remote_path, mode, encoded
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
    fn test_python_encode_chunk_produces_valid_shell() {
        let data = [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0xFF];
        let enc = PythonEncoder;
        let tmp = std::env::temp_dir().join("ordo_test_python");
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
