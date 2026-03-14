//! Keyring Integration - Secure Token Storage
//!
//! Wraps system keyring (Secret Service, Keychain, etc.) to avoid
//! storing tokens in plaintext config files.
//!
//! When compiled without the `keyring-secrets` feature, credential storage
//! falls back to environment variables only.

use anyhow::{anyhow, Result};
use rpassword::read_password;
use std::io::{self, Write};

#[cfg(feature = "keyring-secrets")]
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

    // 2. Check Keyring (if feature enabled)
    #[cfg(feature = "keyring-secrets")]
    {
        use keyring::Entry;
        let entry = Entry::new(SERVICE_NAME, &format!("{}@{}", username, server))?;
        return entry
            .get_password()
            .map_err(|e| anyhow!("Failed to get token: {}", e));
    }

    #[cfg(not(feature = "keyring-secrets"))]
    {
        let _ = (server, username);
        Err(anyhow!(
            "No token found. Set PROGIT_TOKEN env var or rebuild with keyring-secrets feature."
        ))
    }
}

/// Set token in keyring
pub fn set_token(server: &str, username: &str, token: &str) -> Result<()> {
    #[cfg(feature = "keyring-secrets")]
    {
        use keyring::Entry;
        let entry = Entry::new(SERVICE_NAME, &format!("{}@{}", username, server))?;
        return entry
            .set_password(token)
            .map_err(|e| anyhow!("Failed to set token: {}", e));
    }

    #[cfg(not(feature = "keyring-secrets"))]
    {
        let _ = (server, username, token);
        Err(anyhow!(
            "Token storage unavailable. Rebuild with keyring-secrets feature, or use PROGIT_TOKEN env var."
        ))
    }
}

/// Delete token from keyring
pub fn delete_token(server: &str, username: &str) -> Result<()> {
    #[cfg(feature = "keyring-secrets")]
    {
        use keyring::Entry;
        let entry = Entry::new(SERVICE_NAME, &format!("{}@{}", username, server))?;
        return entry
            .delete_password()
            .map_err(|e| anyhow!("Failed to delete token: {}", e));
    }

    #[cfg(not(feature = "keyring-secrets"))]
    {
        let _ = (server, username);
        Ok(()) // No-op: nothing to delete
    }
}

/// Prompt user for token securely (masked input)
pub fn prompt_for_token(server: &str) -> Result<String> {
    print!("Enter Personal Access Token for {}: ", server);
    io::stdout().flush()?;

    let token = read_password()?;
    if token.trim().is_empty() {
        return Err(anyhow!("Token cannot be empty"));
    }

    Ok(token.trim().to_string())
}
