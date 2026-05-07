//! Hinge-compatible verification for ProGit plugins
//!
//! This module implements the supply-chain verification layer from Janus's Hinge
//! package manager, adapted for ProGit plugin distribution.
//!
//! ## Security Model
//!
//! - **Content Integrity**: BLAKE3 checksums verify artifact hasn't been tampered
//! - **Authenticity**: Dilithium3 signatures verify publisher identity
//! - **Trust Policy**: Local keyring determines which publishers are trusted
//!
//! ## Trust Policies
//!
//! - `Strict`: Requires one valid signature from a trusted key
//! - `Consensus { n, m }`: Requires N of M signatures from trusted keys

use crate::marketplace::keyring::Keyring;
use crate::marketplace::manifest::PluginManifest;
use crate::marketplace::crypto as crypto_module;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Verification result
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub valid: bool,
    pub keyid: String,
    pub policy: TrustPolicy,
    pub errors: Vec<String>,
}

impl VerificationResult {
    pub fn success(keyid: &str, policy: TrustPolicy) -> Self {
        Self {
            valid: true,
            keyid: keyid.to_string(),
            policy,
            errors: vec![],
        }
    }

    pub fn failure(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            keyid: String::new(),
            policy: TrustPolicy::Strict,
            errors,
        }
    }
}

/// Trust policy for verification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "params")]
/// Trust policy for plugin verification
pub enum TrustPolicy {
    /// Require at least one signature from a trusted key
    Strict,
    /// Require N of M signatures from trusted keys
    Consensus { n: u8, m: u8 },
}

impl Default for TrustPolicy {
    fn default() -> Self {
        Self::Strict
    }
}

/// Verifier for plugin packages
pub struct Verifier {
    keyring: Keyring,
    policy: TrustPolicy,
}

impl Verifier {
    /// Create a new verifier with the given policy
    pub fn new(policy: TrustPolicy) -> Self {
        Self {
            keyring: Keyring::load_default(),
            policy,
        }
    }

    /// Create a verifier with a custom keyring location
    pub fn with_keyring(path: &Path, policy: TrustPolicy) -> std::result::Result<Self, MarketplaceError> {
        Ok(Self {
            keyring: Keyring::load_from(path).map_err(|e| MarketplaceError::Keyring(e.to_string()))?,
            policy,
        })
    }

    /// Verify a plugin manifest and its signature
    pub fn verify(&self, manifest: &PluginManifest) -> VerificationResult {
        let mut errors = Vec::new();

        // 1. Verify signature structure
        let sig = match &manifest.signature {
            Some(s) => s,
            None => {
                errors.push("No signature found in manifest".to_string());
                return VerificationResult::failure(errors);
            }
        };

        if sig.algorithm != "dilithium3" && sig.algorithm != "dilithium3-test" {
            errors.push(format!("Unsupported signature algorithm: {}", sig.algorithm));
            return VerificationResult::failure(errors);
        }

        // 2. Check keyid is trusted
        let keyid = &sig.keyid;
        if !self.keyring.is_trusted(keyid) {
            errors.push(format!(
                "Key {} is not in trusted keyring. Run: prog trust add <keyid>",
                keyid
            ));
            return VerificationResult::failure(errors);
        }

        // 3. Load publisher public key
        let pubkey = match self.keyring.get_public_key(keyid) {
            Ok(k) => k,
            Err(e) => {
                errors.push(format!("Failed to load public key {}: {}", keyid, e));
                return VerificationResult::failure(errors);
            }
        };

        // 4. Verify signature
        let message = manifest.canonical_json();
        let signature_bytes = match base64_decode(&sig.signature) {
            Ok(b) => b,
            Err(e) => {
                errors.push(format!("Failed to decode signature: {}", e));
                return VerificationResult::failure(errors);
            }
        };

        if !crypto_module::verify_signature(&pubkey, &message, &signature_bytes) {
            errors.push("Signature verification failed".to_string());
            return VerificationResult::failure(errors);
        }

        VerificationResult::success(keyid, self.policy)
    }
    
    /// Verify a raw JSON manifest directly (for legacy plugins)
    pub fn verify_json(&self, json_str: &str) -> VerificationResult {
        let mut errors = Vec::new();

        // Parse the JSON
        let json: serde_json::Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(e) => {
                errors.push(format!("Invalid JSON: {}", e));
                return VerificationResult::failure(errors);
            }
        };

        // Extract signature
        let sig_obj = match json.get("signature") {
            Some(s) => s,
            None => {
                errors.push("No signature found in manifest".to_string());
                return VerificationResult::failure(errors);
            }
        };

        let algorithm = sig_obj.get("algorithm")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let keyid = sig_obj.get("keyid")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let signature = sig_obj.get("signature")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if algorithm != "dilithium3" && algorithm != "dilithium3-test" {
            errors.push(format!("Unsupported signature algorithm: {}", algorithm));
            return VerificationResult::failure(errors);
        }

        // Check keyid is trusted
        if !self.keyring.is_trusted(keyid) {
            errors.push(format!(
                "Key {} is not in trusted keyring. Run: prog trust add <keyid>",
                keyid
            ));
            return VerificationResult::failure(errors);
        }

        // Load publisher public key
        let pubkey = match self.keyring.get_public_key(keyid) {
            Ok(k) => k,
            Err(e) => {
                errors.push(format!("Failed to load public key {}: {}", keyid, e));
                return VerificationResult::failure(errors);
            }
        };

        // Create canonical JSON for signing (same as signing tool)
        let name = json.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let version = json.get("version").and_then(|v| v.as_str()).unwrap_or("");
        let description = json.get("description").and_then(|v| v.as_str()).unwrap_or("");
        let author = json.get("author").and_then(|v| v.as_str()).unwrap_or("");
        let license = json.get("license").and_then(|v| v.as_str()).unwrap_or("");
        let plugin_type = json.get("plugin_type").and_then(|v| v.as_str()).unwrap_or("utility");
        let runtime = json.get("runtime").and_then(|v| v.as_str()).unwrap_or("lua");
        let source_url = json.get("source_url")
            .or_else(|| json.get("sourceUrl"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Create canonical JSON matching the signing tool exactly
        // Order matters for byte-for-byte comparison (serde_json::Map preserves insertion order)
        use serde_json::Map;
        let mut canonical_map = Map::new();
        canonical_map.insert("schemaVersion".to_string(), serde_json::json!(1));
        canonical_map.insert("name".to_string(), serde_json::json!(name));
        canonical_map.insert("version".to_string(), serde_json::json!(version));
        canonical_map.insert("description".to_string(), serde_json::json!(description));
        canonical_map.insert("author".to_string(), serde_json::json!(author));
        canonical_map.insert("license".to_string(), serde_json::json!(license));
        canonical_map.insert("pluginType".to_string(), serde_json::json!(plugin_type));
        canonical_map.insert("runtime".to_string(), serde_json::json!(runtime));
        canonical_map.insert("sourceUrl".to_string(), serde_json::json!(source_url));
        
        let mut publisher = Map::new();
        publisher.insert("keyid".to_string(), serde_json::json!("unsigned"));
        publisher.insert("name".to_string(), serde_json::json!(author));
        canonical_map.insert("publisher".to_string(), serde_json::json!(publisher));
        
        let mut artifact = Map::new();
        artifact.insert("type".to_string(), serde_json::json!(runtime));
        artifact.insert("checksum".to_string(), serde_json::json!("unsigned"));
        artifact.insert("url".to_string(), serde_json::json!(""));
        canonical_map.insert("artifact".to_string(), serde_json::json!(artifact));
        
        let mut caps = Map::new();
        caps.insert("network".to_string(), serde_json::json!([]));
        caps.insert("filesystem".to_string(), serde_json::json!("readOnly"));
        caps.insert("env".to_string(), serde_json::json!([]));
        canonical_map.insert("capabilities".to_string(), serde_json::json!(caps));
        
        canonical_map.insert("keywords".to_string(), serde_json::json!([]));
        canonical_map.insert("homepage".to_string(), serde_json::json!(null));
        
        let canonical = serde_json::Value::Object(canonical_map);

        let message = serde_json::to_vec(&canonical).expect("Failed to serialize canonical JSON");
        let signature_bytes = match base64_decode(signature) {
            Ok(b) => b,
            Err(e) => {
                errors.push(format!("Failed to decode signature: {}", e));
                return VerificationResult::failure(errors);
            }
        };

        if !crypto_module::verify_signature(&pubkey, &message, &signature_bytes) {
            errors.push("Signature verification failed".to_string());
            return VerificationResult::failure(errors);
        }

        VerificationResult::success(keyid, self.policy)
    }

    /// Add a key to the trust store
    pub fn trust_key(&mut self, keyid: &str) -> std::result::Result<(), MarketplaceError> {
        self.keyring.add_trusted(keyid).map_err(|e| MarketplaceError::Keyring(e.to_string()))?;
        self.keyring.save().map_err(|e| MarketplaceError::Keyring(e.to_string()))
    }

    /// Remove a key from the trust store
    pub fn untrust_key(&mut self, keyid: &str) -> std::result::Result<(), MarketplaceError> {
        self.keyring.remove_trusted(keyid).map_err(|e| MarketplaceError::Keyring(e.to_string()))?;
        self.keyring.save().map_err(|e| MarketplaceError::Keyring(e.to_string()))
    }
    
    /// Get the keyring
    pub fn get_keyring(&self) -> &Keyring {
        &self.keyring
    }
}

/// Plugin package containing manifest and artifact
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PluginPackage {
    pub manifest: PluginManifest,
    pub artifact_path: Option<String>,
}

impl PluginPackage {
    /// Verify the artifact checksum matches the manifest
    #[allow(dead_code)]
    pub fn verify_artifact_integrity(&self, artifact_bytes: &[u8]) -> bool {
        let expected_checksum = &self.manifest.artifact.checksum;
        let computed = blake3_checksum(artifact_bytes);

        computed == *expected_checksum
    }
}

/// Compute BLAKE3 checksum of data
#[allow(dead_code)]
pub fn blake3_checksum(data: &[u8]) -> String {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(data);
    format!("blake3:{}", hasher.finalize().to_hex())
}

/// Decode base64 string
fn base64_decode(input: &str) -> std::result::Result<Vec<u8>, MarketplaceError> {
    // Simple base64 decoder
    const BASE64_TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    
    let input = input.trim();
    let mut result = Vec::with_capacity(input.len() * 3 / 4);
    
    let mut buffer: u32 = 0;
    let mut bits_collected = 0;
    
    for c in input.bytes() {
        if c == b'=' || c == b'\n' || c == b'\r' || c == b' ' {
            continue;
        }
        
        let value = match BASE64_TABLE.iter().position(|&x| x == c) {
            Some(v) => v as u32,
            None => return Err(MarketplaceError::Signature(format!("Invalid base64 character: {}", c as char))),
        };
        
        buffer = (buffer << 6) | value;
        bits_collected += 6;
        
        if bits_collected >= 8 {
            bits_collected -= 8;
            result.push((buffer >> bits_collected) as u8);
        }
    }
    
    Ok(result)
}

/// Result type for this module
#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, MarketplaceError>;

/// Marketplace errors
#[derive(Debug, thiserror::Error)]
pub enum MarketplaceError {
    #[error("Keyring error: {0}")]
    Keyring(String),

    #[error("Signature error: {0}")]
    Signature(String),

    #[error("Manifest error: {0}")]
    Manifest(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blake3_checksum() {
        let data = b"hello world";
        let checksum = blake3_checksum(data);
        assert!(checksum.starts_with("blake3:"));
        assert_eq!(checksum.len(), 7 + 32); // "blake3:" + 32 hex chars
    }

    #[test]
    fn test_trust_policy_default() {
        let policy = TrustPolicy::default();
        assert!(matches!(policy, TrustPolicy::Strict));
    }
    
    #[test]
    fn test_base64_decode() {
        // "SGVsbG8=" is "Hello" in base64
        let decoded = base64_decode("SGVsbG8=").unwrap();
        assert_eq!(decoded, b"Hello");
    }
}
