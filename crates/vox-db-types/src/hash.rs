//! SHA3-512 Base32Hex hashing utilities.
use data_encoding::BASE32HEX_NOPAD;
use sha3::{Digest, Sha3_512};

/// Compute a SHA3-512 hash of the given data, returning Base32Hex-encoded string.
pub fn content_hash(data: &[u8]) -> String {
    let mut hasher = Sha3_512::new();
    hasher.update(data);
    let result = hasher.finalize();
    BASE32HEX_NOPAD.encode(&result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_hash() {
        let h1 = content_hash(b"hello world");
        let h2 = content_hash(b"hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_different_data_different_hash() {
        let h1 = content_hash(b"hello");
        let h2 = content_hash(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_length() {
        let h = content_hash(b"test");
        assert!(h.len() > 50);
    }
}
