//! Marketplace CLI commands
//!
//! Implements the trust management and marketplace interaction commands:
//! - `prog trust add <key-url>` - Add a trusted publisher key
//! - `prog trust list` - List trusted keys
//! - `prog trust remove <keyid>` - Remove a trusted key
//! - `prog plugin verify <name>` - Verify plugin integrity

use crate::marketplace::{compute_keyid, Keyring, TrustPolicy, Verifier};
use anyhow::{Context, Result};
use colored::*;

/// Trust subcommands
#[derive(clap::Subcommand, Debug, Clone)]
pub enum TrustAction {
    /// Add a trusted publisher key
    Add {
        /// Key URL (https://...) or KeyID
        key_source: String,
    },
    /// List trusted keys
    List,
    /// Remove a trusted key
    Remove {
        /// KeyID to remove
        keyid: String,
    },
}

/// Handle trust management commands
pub fn handle_trust_command(action: TrustAction) -> Result<()> {
    match action {
        TrustAction::Add { key_source } => {
            trust_add(&key_source)?;
        }
        TrustAction::List => {
            trust_list()?;
        }
        TrustAction::Remove { keyid } => {
            trust_remove(&keyid)?;
        }
    }
    Ok(())
}

/// Add a trusted key
fn trust_add(key_source: &str) -> Result<()> {
    println!("{} Adding trusted key...", "🔑".blue());
    
    let mut verifier = Verifier::new(TrustPolicy::Strict);
    
    // Determine if source is a URL, keyid, or special "core-team" keyword
    let (keyid, public_key) = if key_source.starts_with("https://") || key_source.starts_with("http://") {
        // Fetch public key from URL
        let response = reqwest::blocking::get(key_source)
            .context("Failed to fetch key from URL")?;
        
        let public_key = response.bytes()
            .context("Failed to read key data")?
            .to_vec();
        
        let keyid = compute_keyid(&public_key);
        println!("  {} Key fetched: {}", "✓".green(), keyid);
        
        (keyid, Some(public_key))
    } else if key_source == "progit-core-team" || key_source == "core-team" {
        // Generate test key for core team
        let test_key = b"progit-core-team-test-key-2026";
        let keyid = compute_keyid(test_key);
        println!("  {} Using ProGit Core Team test key: {}", "✓".green(), keyid);
        (keyid, Some(test_key.to_vec()))
    } else if key_source.starts_with("progit-") && key_source.len() < 50 {
        // Treat as test key identifier
        let test_key = key_source.as_bytes();
        let keyid = compute_keyid(test_key);
        println!("  {} Using test key: {}", "✓".green(), keyid);
        (keyid, Some(test_key.to_vec()))
    } else {
        // Assume it's a KeyID to trust directly (no public key available)
        println!("  {} Trusting key by ID only: {}", "ℹ️".yellow(), key_source);
        (key_source.to_string(), None)
    };
    
    // Add key to keyring if we have it
    if let Some(pk) = public_key {
        let mut keyring = Keyring::load_default();
        keyring.add_key(&keyid, &pk)?;
        println!("  {} Public key stored in keyring", "✓".green());
    }
    
    // Add to trust store
    verifier.trust_key(&keyid)?;
    
    println!("{} Key {} is now trusted", "✅".green(), keyid.bold());
    Ok(())
}

/// List trusted keys
fn trust_list() -> Result<()> {
    let keyring = Keyring::load_default();
    let trusted = keyring.list_trusted();
    
    if trusted.is_empty() {
        println!("{} No trusted keys.", "ℹ️".cyan());
        println!("\n  Add a trusted key:");
        println!("  prog trust add https://registry.progit.dev/keys/core-team.pub");
        return Ok(());
    }
    
    println!("{} Trusted Publishers:", "🔑".blue());
    println!("{}", "─".repeat(50));
    
    for keyid in &trusted {
        let trusted_mark = "✓".green();
        
        // Try to get key info
        let info = if keyring.has_key(keyid) {
            format!("in keyring")
        } else {
            "key not found".yellow().to_string()
        };
        
        println!("  {} {} ({})", trusted_mark, keyid.bold(), info.dimmed());
    }
    
    println!("\n  Total: {} trusted key(s)", trusted.len());
    Ok(())
}

/// Remove a trusted key
fn trust_remove(keyid: &str) -> Result<()> {
    let mut verifier = Verifier::new(TrustPolicy::Strict);
    
    if !verifier.get_keyring().is_trusted(keyid) {
        println!("{} Key {} is not trusted", "ℹ️".cyan(), keyid);
        return Ok(());
    }
    
    verifier.untrust_key(keyid)?;
    
    println!("{} Key {} removed from trusted list", "🗑️".green(), keyid.bold());
    Ok(())
}

/// Handle plugin verification
pub fn handle_plugin_verify(name: &str) -> Result<()> {
    let project_root = crate::workspace::find_project_root()?;
    
    // Check multiple locations for plugins (prioritize external plugins directory)
    let search_paths = vec![
        project_root.join("..").join("progit-plugins").join(name),
        project_root.join("plugins").join(name),
    ];
    
    let plugin_dir = search_paths.iter()
        .find(|p| p.exists())
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found", name))?;
    
    // Check if plugin exists
    if !plugin_dir.exists() {
        println!("{} Plugin '{}' not found", "❌".red(), name.bold());
        println!("  Install it with: prog plugin install {}", name);
        return Ok(());
    }
    
    // Look for manifest
    let manifest_path = plugin_dir.join(".progit-plugin.json");
    let legacy_manifest_path = plugin_dir.join("manifest.json");
    
    let manifest_file = if manifest_path.exists() {
        manifest_path
    } else if legacy_manifest_path.exists() {
        legacy_manifest_path
    } else {
        println!("{} No manifest found for '{}'", "⚠️".yellow(), name.bold());
        return Ok(());
    };
    
    println!("{} Verifying plugin '{}'...", "🔍".blue(), name.bold());
    
    // Load manifest
    let manifest_content = std::fs::read_to_string(&manifest_file)?;
    
    // Verify directly from JSON (handles legacy and modern formats)
    let verifier = crate::marketplace::Verifier::new(TrustPolicy::Strict);
    let result = verifier.verify_json(&manifest_content);
    
    if result.valid {
        println!("{} Plugin '{}' verified successfully", "✅".green(), name.bold());
        
        if result.keyid.is_empty() || result.keyid == "unsigned" {
            println!("  ⚠️  Plugin is unsigned");
        } else {
            println!("  Publisher: {}", result.keyid.bold());
            println!("  Policy: {:?}", result.policy);
        }
        
        Ok(())
    } else {
        println!("{} Plugin verification failed:", "❌".red());
        for error in &result.errors {
            println!("  - {}", error.red());
        }
        
        if result.errors.iter().any(|e| e.contains("not in trusted keyring")) {
            println!("\n  Add the publisher key to trust it:");
            println!("  prog trust add https://registry.progit.dev/keys/{}.pub", result.keyid);
        }
        
        Ok(())
    }
}

/// Handle deeplink URLs
pub fn handle_deeplink(url: &str) -> Result<()> {
    // Parse progit:// URL
    let url = url
        .strip_prefix("progit://")
        .ok_or_else(|| anyhow::anyhow!("Invalid deeplink format. Expected: progit://..."))?;
    
    let parts: Vec<&str> = url.split('/').collect();
    
    match parts.as_slice() {
        ["install", plugin_name] => {
            println!("{} Installing '{}' via deeplink...", "🔗".blue(), plugin_name.bold());
            println!("\n  Run this command to install:");
            println!("  {} prog plugin install {}", "→".cyan(), plugin_name);
        }
        ["install", plugin_name, version] => {
            let version = version.strip_prefix('@').unwrap_or(version);
            println!("{} Installing '{}@{}' via deeplink...", "🔗".blue(), plugin_name.bold(), version);
            println!("\n  Run this command to install:");
            println!("  {} prog plugin install {} --version {}", "→".cyan(), plugin_name, version);
        }
        ["update", plugin_name] => {
            println!("{} Updating '{}' via deeplink...", "🔗".blue(), plugin_name.bold());
            println!("\n  Run this command to update:");
            println!("  {} prog plugin update {}", "→".cyan(), plugin_name);
        }
        ["verify", plugin_name] => {
            handle_plugin_verify(plugin_name)?;
        }
        _ => {
            return Err(anyhow::anyhow!("Unknown deeplink action: {}", url));
        }
    }
    
    Ok(())
}
