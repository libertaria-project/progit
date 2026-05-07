//! Cryptographic primitives for Hinge verification
//!
//! ## Algorithm Support
//!
//! - **dilithium3-test**: BLAKE3-based stub for development/testing
//!   (Matches Janus Hinge's approach - any random bytes work as test key)
//! - **dilithium3**: Real post-quantum Dilithium3 (future, requires PQClean)
//!
//! ## Key Identity
//!
//! KeyID = first 16 hex chars of blake3(public_key_bytes)

use blake3::Hasher;

/// Signature algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureAlgorithm {
    /// BLAKE3-based test stub (development)
    Dilithium3Test,
    /// Real Dilithium3 post-quantum (future)
    Dilithium3,
}

impl SignatureAlgorithm {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "dilithium3-test" | "dilithium3test" => Some(Self::Dilithium3Test),
            "dilithium3" => Some(Self::Dilithium3),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dilithium3Test => "dilithium3-test",
            Self::Dilithium3 => "dilithium3",
        }
    }
}

/// Generate a keypair for testing
pub fn generate_test_keypair() -> (Vec<u8>, Vec<u8>) {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut private = vec![0u8; 48];
    let mut public = vec![0u8; 64];

    // Test key generation - in production, use real Dilithium3
    rng.fill(private.as_mut_slice());
    
    // Derive public from private (simple hash for test)
    let mut hasher = Hasher::new();
    hasher.update(&private);
    let digest = hasher.finalize();
    public.copy_from_slice(&digest.as_bytes()[..64.min(digest.as_bytes().len())]);
    
    // Pad to expected size
    while public.len() < 64 {
        public.push(0);
    }

    (private, public)
}

/// Compute KeyID from public key
pub fn compute_keyid(public_key: &[u8]) -> String {
    let mut hasher = Hasher::new();
    hasher.update(public_key);
    let digest = hasher.finalize();
    
    // First 16 hex chars = 8 bytes
    let hex = digest.to_hex();
    hex[..16].to_string()
}

/// Sign a message (test implementation)
pub fn sign_message(private_key: &[u8], message: &[u8]) -> Vec<u8> {
    // Test implementation: HMAC-style using BLAKE3
    let mut hasher = Hasher::new();
    hasher.update(private_key);
    hasher.update(message);
    let digest = hasher.finalize();
    digest.as_bytes().to_vec()
}

/// Verify a signature (test implementation)
pub fn verify_signature(public_key: &[u8], message: &[u8], signature: &[u8]) -> bool {
    // Test implementation: recompute and compare
    let mut hasher = Hasher::new();
    hasher.update(public_key);
    hasher.update(message);
    let computed = hasher.finalize();
    
    // Signature is the expected hash
    signature == computed.as_bytes()
}

/// Sign a message with algorithm
pub fn sign(algorithm: SignatureAlgorithm, private_key: &[u8], message: &[u8]) -> Vec<u8> {
    match algorithm {
        SignatureAlgorithm::Dilithium3Test => sign_message(private_key, message),
        SignatureAlgorithm::Dilithium3 => {
            // Future: Real Dilithium3
            // For now, fall back to test implementation
            sign_message(private_key, message)
        }
    }
}

/// Verify a signature with algorithm
pub fn verify(algorithm: SignatureAlgorithm, public_key: &[u8], message: &[u8], signature: &[u8]) -> bool {
    match algorithm {
        SignatureAlgorithm::Dilithium3Test => verify_signature(public_key, message, signature),
        SignatureAlgorithm::Dilithium3 => {
            // Future: Real Dilithium3
            verify_signature(public_key, message, signature)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyid_derivation() {
        let pk1 = b"test public key 1";
        let pk2 = b"test public key 2";
        
        let keyid1 = compute_keyid(pk1);
        let keyid2 = compute_keyid(pk2);
        
        assert_eq!(keyid1.len(), 16);
        assert_ne!(keyid1, keyid2);
    }

    #[test]
    fn test_sign_verify() {
        let (private, public) = generate_test_keypair();
        let message = b"Hello, Hinge!";
        
        let signature = sign(SignatureAlgorithm::Dilithium3Test, &private, message);
        assert!(verify(SignatureAlgorithm::Dilithium3Test, &public, message, &signature));
    }

    #[test]
    fn test_signature_rejection() {
        let (private, public) = generate_test_keypair();
        let message = b"Hello, Hinge!";
        let wrong_message = b"Hello, Intruder!";
        
        let signature = sign(SignatureAlgorithm::Dilithium3Test, &private, message);
        assert!(!verify(SignatureAlgorithm::Dilithium3Test, &public, wrong_message, &signature));
    }
}
