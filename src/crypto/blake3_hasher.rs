use crate::Result;
use blake3::Hasher;

pub struct Blake3Hasher;

impl Blake3Hasher {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }
    
    pub fn hash_content(&self, content: &[u8]) -> String {
        let mut hasher = Hasher::new();
        hasher.update(content);
        hasher.finalize().to_hex().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blake3_hasher_new() {
        let hasher = Blake3Hasher::new().unwrap();
        assert!(hasher.hash_content(b"test").len() > 0);
    }

    #[test]
    fn test_hash_content_consistency() {
        let hasher = Blake3Hasher::new().unwrap();
        let content = b"Hello, World!";
        let hash1 = hasher.hash_content(content);
        let hash2 = hasher.hash_content(content);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_content_different_inputs() {
        let hasher = Blake3Hasher::new().unwrap();
        let hash1 = hasher.hash_content(b"Hello");
        let hash2 = hasher.hash_content(b"World");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_content_empty() {
        let hasher = Blake3Hasher::new().unwrap();
        let hash = hasher.hash_content(b"");
        assert_eq!(hash.len(), 128); // Blake3 produces 64-byte hash, hex encoded = 128 chars
    }

    #[test]
    fn test_hash_content_large_input() {
        let hasher = Blake3Hasher::new().unwrap();
        let large_content = vec![0u8; 10000];
        let hash = hasher.hash_content(&large_content);
        assert_eq!(hash.len(), 128);
    }
}
