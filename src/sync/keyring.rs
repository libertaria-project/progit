//! Keyring Integration - Secure Token Storage
//!
//! Wraps system keyring (Secret Service, Keychain, etc.) to avoid
//! storing tokens in plaintext config files.

use anyhow::{anyhow, Result};
use keyring::Entry;
use rpassword::read_password;
use std::io::{self, Write};

const SERVICE_NAME: &str = "progit";

/// Get token from keyring or environment
pub fn get_token(server: &str, username: &str) -> Result<String> {
    // 1. Check Env Vars (Override)
    if let Ok(token) = std::env::var("PROGIT_TOKEN") {
        return Ok(token);
    }
    if let Ok(token) = std::env::var("GITLAB_TOKEN") {
        return Ok(token);
    }
    if let Ok(token) = std::env::var("FORGEJO_TOKEN") {
        return Ok(token);
    }

    // 2. Check Keyring
    let entry = Entry::new(SERVICE_NAME, &format!("{}@{}", username, server))?;
    entry
        .get_password()
        .map_err(|e| anyhow!("Failed to get token: {}", e))
}

/// Set token in keyring
pub fn set_token(server: &str, username: &str, token: &str) -> Result<()> {
    let entry = Entry::new(SERVICE_NAME, &format!("{}@{}", username, server))?;
    entry
        .set_password(token)
        .map_err(|e| anyhow!("Failed to set token: {}", e))
}

/// Delete token from keyring
pub fn delete_token(server: &str, username: &str) -> Result<()> {
    let entry = Entry::new(SERVICE_NAME, &format!("{}@{}", username, server))?;
    entry
        .delete_password()
        .map_err(|e| anyhow!("Failed to delete token: {}", e))
}

/// Prompt user for token securely (masked input)
pub fn prompt_for_token(server: &str) -> Result<String> {
    print!("🔑 Enter Personal Access Token for {}: ", server);
    io::stdout().flush()?;

    let token = read_password()?;
    if token.trim().is_empty() {
        return Err(anyhow!("Token cannot be empty"));
    }

    Ok(token.trim().to_string())
}
