use openssl::sha;

/// Create a sha256 hash from a reference of bytes. Return bytes.
pub fn sha256(data: &[u8]) -> Vec<u8> {
    let mut hasher = sha::Sha256::new();
    hasher.update(data);
    hasher.finish().to_vec()
}

/// Create a sha384 hash from a reference of bytes. Return bytes.
pub fn sha384(data: &[u8]) -> Vec<u8> {
    let mut hasher = sha::Sha384::new();
    hasher.update(data);
    hasher.finish().to_vec()
}
