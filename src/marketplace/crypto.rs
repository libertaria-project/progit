//! Cryptographic primitives for Hinge verification
//!
//! This module provides:
//! - BLAKE3 checksums for content integrity
//! - ML-DSA/Dilithium3 signatures (FIPS 204)
//!
//! The real implementation uses `crystals-dilithium` for post-quantum
//! signature verification. A test implementation is provided for
//! development and testing.

use std::path::Path;

/// Signature algorithm identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    /// ML-DSA-44 (Dilithium3, FIPS 204)
    Dilithium3,
    /// Test/dummy signature (BLAKE3-based, 32 byte signatures)
    #[cfg(test)]
    Test,
}

impl Algorithm {
    pub fn as_str(&self) -> &'static str {
        match self {
            Algorithm::Dilithium3 => "dilithium3",
            #[cfg(test)]
            Algorithm::Test => "dilithium3-test",
        }
    }
}

/// Keypair for signing and verification (Dilithium3)
pub struct KeyPair {
    pub public_key: Vec<u8>,
    secret_key: Vec<u8>,
}

/// Generate a new Dilithium3 keypair
pub fn generate_keypair() -> KeyPair {
    use crystals_dilithium::dilithium3::Keypair;

    let keypair = Keypair::generate(None).expect("Failed to generate Dilithium3 keypair");

    KeyPair {
        public_key: keypair.public.to_bytes().to_vec(),
        secret_key: keypair.to_bytes().to_vec(),
    }
}

/// Sign data with Dilithium3
pub fn sign(data: &[u8], secret_key: &[u8]) -> Vec<u8> {
    use crystals_dilithium::dilithium3::Keypair;

    let keypair = Keypair::from_bytes(secret_key).expect("Invalid secret key");
    let sig = keypair.sign(data);
    sig.to_vec()
}

/// Verify Dilithium3 signature
pub fn verify(public_key: &[u8], data: &[u8], signature: &[u8]) -> bool {
    use crystals_dilithium::dilithium3::PublicKey;

    match PublicKey::from_bytes(public_key) {
        Ok(pk) => pk.verify(data, signature),
        Err(_) => false,
    }
}

/// Compute BLAKE3 checksum of data
pub fn blake3_checksum(data: &[u8]) -> String {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(data);
    format!("blake3:{}", hasher.finalize().to_hex())
}

/// Compute KeyID from public key (BLAKE3 hash, first 16 hex chars)
pub fn compute_keyid(public_key: &[u8]) -> String {
    blake3_checksum(public_key)[5..21].to_string()
}

/// Save keypair to files
pub fn save_keypair(
    keypair: &KeyPair,
    secret_path: &Path,
    public_path: &Path,
) -> std::io::Result<()> {
    std::fs::write(secret_path, &keypair.secret_key)?;
    std::fs::write(public_path, &keypair.public_key)?;
    Ok(())
}

/// Load keypair from files
pub fn load_keypair(secret_path: &Path, public_path: &Path) -> std::io::Result<KeyPair> {
    let secret_key = std::fs::read(secret_path)?;
    let public_key = std::fs::read(public_path)?;
    Ok(KeyPair {
        public_key,
        secret_key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blake3_checksum() {
        let data = b"hello world";
        let checksum = blake3_checksum(data);
        assert!(checksum.starts_with("blake3:"));
        // "blake3:" (5) + blake3 hash (256 bits = 64 hex chars) = 69
        // But blake3::Hasher outputs 32 bytes = 64 hex chars
        assert!(checksum.len() >= 69);
    }

    #[test]
    fn test_keyid() {
        let key = b"test public key";
        let keyid = compute_keyid(key);
        assert_eq!(keyid.len(), 16);
    }

    #[test]
    fn test_dilithium3_sign_verify() {
        let keypair = generate_keypair();
        let data = b"test message";

        let signature = sign(data, &keypair.secret_key);
        assert!(verify(&keypair.public_key, data, &signature));

        // Verify fails with wrong data
        assert!(!verify(&keypair.public_key, b"wrong data", &signature));
    }
}
