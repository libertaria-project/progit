//! Plugin signing tool
//!
//! Signs plugin manifests with a publisher key.
//! Usage: cargo run --bin sign-plugin -- <manifest.json>

use blake3::Hasher;
use std::fs;
use std::path::PathBuf;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Plugin manifest path (manifest.json or .progit-plugin.json)
    manifest: PathBuf,
    
    /// Output path for signed manifest (default: overwrite original)
    #[arg(short, long)]
    output: Option<PathBuf>,
    
    /// Force sign even if already signed
    #[arg(short, long)]
    force: bool,
    
    /// Show the keyid without signing
    #[arg(long)]
    keyid_only: bool,
}

fn main() {
    let args = Args::parse();
    
    // Load manifest
    let manifest_content = fs::read_to_string(&args.manifest)
        .expect("Failed to read manifest");
    
    // Parse to validate
    let mut manifest: serde_json::Value = serde_json::from_str(&manifest_content)
        .expect("Invalid JSON manifest");
    
    // Check if already signed
    if let Some(sig) = manifest.get("signature") {
        if !args.force {
            println!("⚠️  Plugin is already signed. Use --force to re-sign.");
            println!("   Current signature: {:?}", sig);
            return;
        }
    }
    
    // Create a minimal canonical manifest for signing
    // Use LinkedHashMap to preserve insertion order
    use std::collections::BTreeMap;
    
    let name = manifest.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let version = manifest.get("version").and_then(|v| v.as_str()).unwrap_or("");
    let description = manifest.get("description").and_then(|v| v.as_str()).unwrap_or("");
    let author = manifest.get("author").and_then(|v| v.as_str()).unwrap_or("");
    let license = manifest.get("license").and_then(|v| v.as_str()).unwrap_or("");
    let plugin_type = manifest.get("plugin_type").and_then(|v| v.as_str()).unwrap_or("utility");
    let runtime = manifest.get("runtime").and_then(|v| v.as_str()).unwrap_or("lua");
    let source_url = manifest.get("source_url").or_else(|| manifest.get("sourceUrl"))
        .and_then(|v| v.as_str()).unwrap_or("");
    
    // Create canonical JSON with ordered fields (using BTreeMap for deterministic order)
    let mut canonical = BTreeMap::new();
    canonical.insert("schemaVersion", serde_json::json!(1));
    canonical.insert("name", serde_json::json!(name));
    canonical.insert("version", serde_json::json!(version));
    canonical.insert("description", serde_json::json!(description));
    canonical.insert("author", serde_json::json!(author));
    canonical.insert("license", serde_json::json!(license));
    canonical.insert("pluginType", serde_json::json!(plugin_type));
    canonical.insert("runtime", serde_json::json!(runtime));
    canonical.insert("sourceUrl", serde_json::json!(source_url));
    
    let mut publisher = BTreeMap::new();
    publisher.insert("keyid", serde_json::json!("unsigned"));
    publisher.insert("name", serde_json::json!(author));
    canonical.insert("publisher", serde_json::json!(publisher));
    
    let mut artifact = BTreeMap::new();
    artifact.insert("type", serde_json::json!(runtime));
    artifact.insert("checksum", serde_json::json!("unsigned"));
    artifact.insert("url", serde_json::json!(""));
    canonical.insert("artifact", serde_json::json!(artifact));
    
    let mut caps = BTreeMap::new();
    caps.insert("network", serde_json::json!([]));
    caps.insert("filesystem", serde_json::json!("readOnly"));
    caps.insert("env", serde_json::json!([]));
    canonical.insert("capabilities", serde_json::json!(caps));
    
    canonical.insert("keywords", serde_json::json!([]));
    canonical.insert("homepage", serde_json::json!(null));
    
    // Serialize to canonical JSON bytes
    let canonical_json = serde_json::to_vec(&canonical)
        .expect("Failed to serialize canonical form");
    
    // For now, use a deterministic test key
    let private_key = b"progit-core-team-test-key-2026";
    
    // Sign
    let signature = sign_message(private_key, &canonical_json);
    let signature_b64 = base64_encode(&signature);
    
    // Compute keyid
    let computed_keyid = compute_keyid(private_key);
    
    if args.keyid_only {
        println!("KeyID: {}", computed_keyid);
        return;
    }
    
    // Add signature to original manifest (preserving all fields)
    if let Some(obj) = manifest.as_object_mut() {
        obj.insert("signature".to_string(), serde_json::json!({
            "algorithm": "dilithium3-test",
            "keyid": computed_keyid,
            "signature": signature_b64
        }));
    }
    
    // Write signed manifest
    let output_path = args.output.unwrap_or_else(|| args.manifest.clone());
    
    let signed_json = serde_json::to_string_pretty(&manifest)
        .expect("Failed to serialize signed manifest");
    fs::write(&output_path, signed_json)
        .expect("Failed to write signed manifest");
    
    println!("✅ Plugin signed successfully");
    println!("   Input: {}", args.manifest.display());
    println!("   Output: {}", output_path.display());
    println!("   KeyID: {}", computed_keyid);
    println!("   Signature: {} bytes", signature.len());
}

/// Sign a message with test implementation (BLAKE3-based)
fn sign_message(private_key: &[u8], message: &[u8]) -> Vec<u8> {
    let mut hasher = Hasher::new();
    hasher.update(private_key);
    hasher.update(message);
    let digest = hasher.finalize();
    digest.as_bytes().to_vec()
}

/// Compute KeyID from key bytes
fn compute_keyid(key: &[u8]) -> String {
    let mut hasher = Hasher::new();
    hasher.update(key);
    let digest = hasher.finalize();
    digest.to_hex()[..16].to_string()
}

/// Simple base64 encoder
fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    
    let mut result = String::new();
    let mut i = 0;
    
    while i < data.len() {
        let b0 = data[i] as usize;
        let b1 = if i + 1 < data.len() { data[i + 1] as usize } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] as usize } else { 0 };
        
        result.push(TABLE[b0 >> 2] as char);
        result.push(TABLE[((b0 & 0x03) << 4) | (b1 >> 4)] as char);
        
        if i + 1 < data.len() {
            result.push(TABLE[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }
        
        if i + 2 < data.len() {
            result.push(TABLE[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
        
        i += 3;
    }
    
    result
}
