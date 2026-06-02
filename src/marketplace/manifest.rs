//! Plugin manifest schema with Hinge-compatible signatures
//!
//! ## Manifest Structure
//!
//! A plugin manifest is a JSON file that contains:
//! - Plugin identity (name, version, author)
//! - Publisher information with KeyID
//! - Artifact reference with checksum
//! - Capability declarations
//! - Cryptographic signature

use serde::{Deserialize, Serialize};

/// Plugin manifest - the "trust contract" for a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    /// Schema version for future compatibility
    pub schema_version: u32,
    
    /// Plugin identity
    pub name: String,
    pub version: String,
    pub description: String,
    
    /// Author information
    pub author: String,
    
    /// License (SPDX format)
    pub license: String,
    
    /// Plugin category
    #[serde(rename = "pluginType")]
    pub plugin_type: PluginType,
    
    /// Runtime (lua or wasm)
    pub runtime: Runtime,
    
    /// Source URL for the plugin repository
    #[serde(rename = "sourceUrl")]
    pub source_url: String,
    
    /// Publisher identity (Hinge-compatible)
    pub publisher: Publisher,
    
    /// Artifact information
    pub artifact: Artifact,
    
    /// Declared capabilities
    pub capabilities: Capabilities,
    
    /// Cryptographic signature
    pub signature: Option<Signature>,
    
    /// Optional metadata
    #[serde(default)]
    pub keywords: Vec<String>,
    
    /// Optional homepage
    #[serde(default)]
    pub homepage: Option<String>,
}

impl PluginManifest {
    /// Get the canonical JSON representation for signing
    /// Excludes the signature field itself
    pub fn canonical_json(&self) -> Vec<u8> {
        // Create a minimal canonical manifest matching the signing tool's format
        let canonical = serde_json::json!({
            "schemaVersion": self.schema_version,
            "name": &self.name,
            "version": &self.version,
            "description": &self.description,
            "author": &self.author,
            "license": &self.license,
            "pluginType": serde_json::to_value(&self.plugin_type).unwrap(),
            "runtime": serde_json::to_value(&self.runtime).unwrap(),
            "sourceUrl": &self.source_url,
            "publisher": {
                "keyid": &self.publisher.keyid,
                "name": &self.publisher.name,
            },
            "artifact": {
                "type": serde_json::to_value(&self.artifact.r#type).unwrap(),
                "checksum": &self.artifact.checksum,
                "url": &self.artifact.url,
            },
            "capabilities": {
                "network": &self.capabilities.network,
                "filesystem": serde_json::to_value(&self.capabilities.filesystem).unwrap(),
                "env": &self.capabilities.env,
            },
            "keywords": &self.keywords,
            "homepage": &self.homepage,
        });
        serde_json::to_vec(&canonical).unwrap()
    }
}

/// Publisher identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Publisher {
    /// KeyID (first 16 hex of blake3(public_key))
    pub keyid: String,
    
    /// Publisher name
    pub name: String,
    
    /// Optional DID for decentralized identity
    #[serde(default)]
    pub did: Option<String>,
}

/// Artifact reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    /// Runtime type
    pub r#type: Runtime,
    
    /// BLAKE3 checksum
    pub checksum: String,
    
    /// URL to download the artifact
    pub url: String,
    
    /// Optional file size
    #[serde(default)]
    pub size: Option<u64>,
}

/// Declared capabilities for security sandboxing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    /// Allowed network destinations
    #[serde(default)]
    pub network: Vec<String>,
    
    /// Filesystem access level
    #[serde(default)]
    pub filesystem: FilesystemAccess,
    
    /// Required environment variables (empty = none)
    #[serde(default)]
    pub env: Vec<String>,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            network: vec![],
            filesystem: FilesystemAccess::ReadOnly,
            env: vec![],
        }
    }
}

/// Filesystem access level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FilesystemAccess {
    /// No filesystem access
    None,
    /// Read-only access
    ReadOnly,
    /// Read-write access to plugin directory only
    PluginDir,
    /// Full read-write access
    ReadWrite,
}

impl Default for FilesystemAccess {
    fn default() -> Self {
        Self::ReadOnly
    }
}

/// Cryptographic signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    /// Signature algorithm
    pub algorithm: String,
    
    /// KeyID of the signing key
    pub keyid: String,
    
    /// Base64-encoded signature
    pub signature: String,
}

/// Plugin type categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    Integration,
    Analytics,
    Utility,
    Theme,
    Automation,
    Renderer,
    Ai,
}

/// Plugin runtime
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Runtime {
    Lua,
    Wasm,
}

/// Minimal manifest for signing (excludes signature itself)
#[derive(Serialize)]
#[allow(dead_code)]
struct CanonicalManifest<'a> {
    schema_version: u32,
    name: &'a str,
    version: &'a str,
    description: &'a str,
    author: &'a str,
    license: &'a str,
    plugin_type: &'a PluginType,
    runtime: &'a Runtime,
    source_url: &'a str,
    publisher: &'a Publisher,
    artifact: &'a Artifact,
    capabilities: &'a Capabilities,
    keywords: &'a Vec<String>,
    homepage: &'a Option<String>,
}

/// Legacy manifest for compatibility with existing plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyPluginManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    pub author: String,
    pub license: String,
    #[serde(alias = "pluginType", alias = "plugin_type")]
    pub plugin_type: String,
    #[serde(alias = "runtime")]
    pub runtime: String,
    #[serde(alias = "sourceUrl", alias = "source_url")]
    pub source_url: Option<String>,
    #[serde(default)]
    pub hooks: Option<Vec<String>>,
    #[serde(default)]
    pub capabilities: Option<serde_json::Value>,
    #[serde(default)]
    pub signature: Option<Signature>,
}

impl TryFrom<LegacyPluginManifest> for PluginManifest {
    type Error = String;

    fn try_from(legacy: LegacyPluginManifest) -> Result<Self, Self::Error> {
        let author_name = legacy.author.clone();
        Ok(PluginManifest {
            schema_version: 1,
            name: legacy.name,
            version: legacy.version,
            description: legacy.description.unwrap_or_default(),
            author: legacy.author.clone(),
            license: legacy.license,
            plugin_type: match legacy.plugin_type.as_str() {
                "integration" => PluginType::Integration,
                "analytics" => PluginType::Analytics,
                "utility" => PluginType::Utility,
                "theme" => PluginType::Theme,
                "automation" => PluginType::Automation,
                "renderer" => PluginType::Renderer,
                "ai" => PluginType::Ai,
                _ => PluginType::Utility,
            },
            runtime: match legacy.runtime.as_str() {
                "wasm" => Runtime::Wasm,
                _ => Runtime::Lua,
            },
            source_url: legacy.source_url.unwrap_or_default(),
            publisher: Publisher {
                keyid: legacy.signature.as_ref()
                    .map(|s| s.keyid.clone())
                    .unwrap_or_else(|| "unsigned".to_string()),
                name: author_name,
                did: None,
            },
            artifact: Artifact {
                r#type: Runtime::Lua,
                checksum: "unsigned".to_string(),
                url: String::new(),
                size: None,
            },
            capabilities: Capabilities::default(),
            signature: legacy.signature,
            keywords: vec![],
            homepage: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_serialization() {
        let manifest = PluginManifest {
            schema_version: 1,
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "A test plugin".to_string(),
            author: "Test Author".to_string(),
            license: "MIT".to_string(),
            plugin_type: PluginType::Utility,
            runtime: Runtime::Lua,
            source_url: "https://example.com/test-plugin".to_string(),
            publisher: Publisher {
                keyid: "a1b2c3d4e5f6a7b8".to_string(),
                name: "Test Author".to_string(),
                did: None,
            },
            artifact: Artifact {
                r#type: Runtime::Lua,
                checksum: "blake3:abc123".to_string(),
                url: "https://example.com/test-plugin/main.lua".to_string(),
                size: Some(1024),
            },
            capabilities: Capabilities::default(),
            signature: None,
            keywords: vec!["test".to_string()],
            homepage: None,
        };

        let json = serde_json::to_string_pretty(&manifest).unwrap();
        assert!(json.contains("test-plugin"));
        assert!(json.contains("a1b2c3d4e5f6a7b8"));
    }

    #[test]
    fn test_canonical_json() {
        let manifest = PluginManifest {
            schema_version: 1,
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: "desc".to_string(),
            author: "author".to_string(),
            license: "MIT".to_string(),
            plugin_type: PluginType::Utility,
            runtime: Runtime::Lua,
            source_url: "https://example.com".to_string(),
            publisher: Publisher {
                keyid: "keyid123".to_string(),
                name: "name".to_string(),
                did: None,
            },
            artifact: Artifact {
                r#type: Runtime::Lua,
                checksum: "blake3:xyz".to_string(),
                url: "https://example.com/file.lua".to_string(),
                size: None,
            },
            capabilities: Capabilities::default(),
            signature: None,
            keywords: vec![],
            homepage: None,
        };

        let canonical = manifest.canonical_json();
        let parsed: serde_json::Value = serde_json::from_slice(&canonical).unwrap();
        
        // Should not contain signature
        assert!(!parsed.as_object().unwrap().contains_key("signature"));
    }
}
