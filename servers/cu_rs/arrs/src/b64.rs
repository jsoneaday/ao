use base64ct::{Base64UrlUnpadded, Encoding};
// use base64ct::Error as B64Error;
use std::error::Error;

/// Input a byte reference (&[u8]), and encode it into a String.
pub fn b64_encode(bytes: &[u8]) -> String {
    Base64UrlUnpadded::encode_string(bytes)
}

/// Input a string reference (a slice &str), and decode it into bytes
/// which are represented by a `Vec<u8>`.
pub fn b64_decode(str: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let res = Base64UrlUnpadded::decode_vec(str);
    match res {
        Ok(bytes) => Ok(bytes),
        Err(e) => Err(e.to_string().into())
    }
}
