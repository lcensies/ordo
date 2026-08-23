pub mod base64;
pub mod basenc;
pub mod perl;
pub mod printf;
pub mod python;

pub trait Encoder {
    fn name(&self) -> &str;
    fn encode_chunk(&self, data: &[u8], remote_path: &str, first: bool) -> String;
    fn verify_cmd(&self, remote_path: &str, expected_hex: &str) -> String;
}
