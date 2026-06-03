//! Local keyring for trusted publisher keys
//!
//! ## Keyring Structure
//!
//! The keyring is stored in `~/.progit/keyring/` and contains:
//! - `<keyid>.pub` - Public key files
//! - `keyring.kdl` - Trust relationships
//!
//! ## KeyID
//!
//! KeyID = first 16 hex chars of blake3(public_key_bytes)
//! This is content-addressed, meaning a key's identity is derived from its content.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Keyring for managing trusted publisher keys
#[derive(Debug, Clone)]
pub struct Keyring {
    /// Directory containing the keyring
    path: PathBuf,

    /// Set of trusted KeyIDs
    trusted: HashSet<String>,
}

impl Keyring {
    /// Load the default keyring from ~/.progit/keyring/
    pub fn load_default() -> Self {
        let path = Self::default_path();
        Self::load_from(&path).unwrap_or_else(|_| Self::new(path))
    }

    /// Get the default keyring path
    fn default_path() -> PathBuf {
        dirs::home_dir()
            .map(|h| h.join(".progit/keyring"))
            .unwrap_or_else(|| PathBuf::from(".progit/keyring"))
    }

    /// Load keyring from a specific path
    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(KeyringError::NotFound(path.display().to_string()));
        }

        let mut keyring = Self::new(path.to_path_buf());

        // Load trust relationships
        let trust_file = path.join("keyring.kdl");
        if trust_file.exists() {
            keyring.load_trust_file(&trust_file)?;
        }

        Ok(keyring)
    }

    /// Create a new empty keyring
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            trusted: HashSet::new(),
        }
    }

    /// Load trust relationships from KDL file
    fn load_trust_file(&mut self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)?;

        // Simple KDL parsing for trust entries
        // Format: trusted "<keyid>"
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("trusted") && line.contains('"') {
                if let Some(keyid) = line.split('"').nth(1) {
                    self.trusted.insert(keyid.to_string());
                }
            }
        }

        Ok(())
    }

    /// Save the keyring to disk
    pub fn save(&self) -> Result<()> {
        // Ensure directory exists
        fs::create_dir_all(&self.path)?;

        // Write trust file
        let trust_file = self.path.join("keyring.kdl");
        let mut content = String::new();
        content.push_str("# ProGit Keyring\n");
        content.push_str("# Edit manually or use: prog trust add/remove\n\n");

        for keyid in &self.trusted {
            content.push_str(&format!("trusted \"{}\"\n", keyid));
        }

        fs::write(&trust_file, content)?;

        Ok(())
    }

    /// Add a trusted key by KeyID
    pub fn add_trusted(&mut self, keyid: &str) -> Result<()> {
        self.trusted.insert(keyid.to_string());
        self.save()
    }

    /// Remove a trusted key by KeyID
    pub fn remove_trusted(&mut self, keyid: &str) -> Result<()> {
        self.trusted.remove(keyid);
        self.save()
    }

    /// Check if a KeyID is trusted
    pub fn is_trusted(&self, keyid: &str) -> bool {
        self.trusted.contains(keyid)
    }

    /// List all trusted keys
    pub fn list_trusted(&self) -> Vec<&String> {
        self.trusted.iter().collect()
    }

    /// Add a public key to the keyring
    pub fn add_key(&mut self, keyid: &str, public_key: &[u8]) -> Result<()> {
        fs::create_dir_all(&self.path)?;

        let key_file = self.path.join(format!("{}.pub", keyid));
        fs::write(&key_file, public_key)?;

        Ok(())
    }

    /// Get a public key by KeyID
    pub fn get_public_key(&self, keyid: &str) -> Result<Vec<u8>> {
        let key_file = self.path.join(format!("{}.pub", keyid));

        if !key_file.exists() {
            return Err(KeyringError::KeyNotFound(keyid.to_string()));
        }

        fs::read(&key_file).map_err(|e| KeyringError::Io(e.to_string()))
    }

    /// Check if a key exists in the keyring
    pub fn has_key(&self, keyid: &str) -> bool {
        self.path.join(format!("{}.pub", keyid)).exists()
    }

    /// List all keys in the keyring
    pub fn list_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "pub") {
                    if let Some(keyid) = path.file_stem().and_then(|s| s.to_str()) {
                        keys.push(keyid.to_string());
                    }
                }
            }
        }

        keys.sort();
        keys
    }
}

/// Keyring-specific error type
#[derive(Debug, thiserror::Error)]
pub enum KeyringError {
    #[error("Keyring not found at {0}")]
    NotFound(String),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, KeyringError>;

impl From<std::io::Error> for KeyringError {
    fn from(e: std::io::Error) -> KeyringError {
        KeyringError::Io(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_keyring_trust() {
        let tmp = TempDir::new().unwrap();
        let mut keyring = Keyring::new(tmp.path().to_path_buf());

        keyring.add_trusted("a1b2c3d4e5f6a7b8").unwrap();

        assert!(keyring.is_trusted("a1b2c3d4e5f6a7b8"));
        assert!(!keyring.is_trusted("0000000000000000"));
    }
}
