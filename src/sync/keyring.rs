//! Keyring Integration - Secure Token Storage
//!
//! Wraps system keyring (Secret Service, Keychain, etc.) to avoid
//! storing tokens in plaintext config files.
//!
//! When compiled without the `keyring-secrets` feature, credential storage
//! falls back to environment variables only.

use anyhow::{anyhow, Context, Result};
use rpassword::read_password;
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

#[cfg(feature = "keyring-secrets")]
const SERVICE_NAME: &str = "progit";

/// Get token from keyring, environment, or a configured token file.
pub fn get_token(server: &str, username: &str) -> Result<String> {
    // 1. Check Env Vars (Override)
    if let Some(token) = env_token("PROGIT_TOKEN") {
        return Ok(token);
    }
    if let Some(token) = env_token("GITLAB_TOKEN") {
        return Ok(token);
    }
    if let Some(token) = env_token("FORGEJO_TOKEN") {
        return Ok(token);
    }

    // 2. Check token files. Files may contain either a raw token or KEY=value.
    if let Some(token) = token_from_file_env("PROGIT_TOKEN_FILE", "PROGIT_TOKEN_ENV_VAR")? {
        return Ok(token);
    }
    if let Some(token) = token_from_file_env("GITLAB_TOKEN_FILE", "GITLAB_TOKEN_ENV_VAR")? {
        return Ok(token);
    }
    if let Some(token) = token_from_file_env("FORGEJO_TOKEN_FILE", "FORGEJO_TOKEN_ENV_VAR")? {
        return Ok(token);
    }

    // Compatibility with existing deployment env-file conventions.
    if let Some(token) = token_from_file_env("TOKEN_ENV_FILE", "TOKEN_ENV_VAR")? {
        return Ok(token);
    }

    // 3. Check Keyring (if feature enabled)
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
            "No token found. Set PROGIT_TOKEN, FORGEJO_TOKEN, GITLAB_TOKEN, or *_TOKEN_FILE."
        ))
    }
}

fn env_token(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty())
}

fn token_from_file_env(path_env: &str, var_env: &str) -> Result<Option<String>> {
    let path = match std::env::var(path_env) {
        Ok(path) if !path.trim().is_empty() => expand_home(path.trim()),
        _ => return Ok(None),
    };
    let requested_var = std::env::var(var_env).ok();
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read token file {}", path.display()))?;

    parse_token_file_content(&content, requested_var.as_deref())
        .with_context(|| format!("Failed to parse token file {}", path.display()))
        .map(Some)
}

fn expand_home(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

fn parse_token_file_content(content: &str, requested_var: Option<&str>) -> Result<String> {
    let mut raw_token = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key
                .trim()
                .strip_prefix("export ")
                .unwrap_or(key.trim())
                .trim();
            if requested_var.map_or(true, |requested| key == requested) {
                let token = normalize_token_value(value);
                if token.is_empty() {
                    return Err(anyhow!("Token value for {} is empty", key));
                }
                return Ok(token);
            }
            continue;
        }

        if requested_var.is_none() && raw_token.is_none() {
            raw_token = Some(line.to_string());
        }
    }

    raw_token
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(|| match requested_var {
            Some(var) => anyhow!("Token variable {} not found", var),
            None => anyhow!("Token file does not contain a token"),
        })
}

fn normalize_token_value(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
        {
            return value[1..value.len() - 1].trim().to_string();
        }
    }
    value.to_string()
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
            "Token storage unavailable. Rebuild with keyring-secrets feature, or use PROGIT_TOKEN/PROGIT_TOKEN_FILE."
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

#[cfg(test)]
mod tests {
    use super::{parse_token_file_content, token_from_file_env};

    #[test]
    fn parses_raw_token_file_content() {
        let token = parse_token_file_content("abc123\n", None).unwrap();
        assert_eq!(token, "abc123");
    }

    #[test]
    fn parses_env_file_content_by_requested_var() {
        let token = parse_token_file_content(
            "OTHER=nope\nFORGEJO_TOKEN=\"abc123\"\n",
            Some("FORGEJO_TOKEN"),
        )
        .unwrap();
        assert_eq!(token, "abc123");
    }

    #[test]
    fn parses_first_env_assignment_when_var_is_unspecified() {
        let token = parse_token_file_content("export FORGEJO_TOKEN='abc123'\n", None).unwrap();
        assert_eq!(token, "abc123");
    }

    #[test]
    fn reads_token_from_configured_file_env() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("token.env");
        std::fs::write(&path, "abc123\n").unwrap();

        std::env::set_var("PROGIT_TEST_TOKEN_FILE", &path);
        std::env::remove_var("PROGIT_TEST_TOKEN_ENV_VAR");

        let token =
            token_from_file_env("PROGIT_TEST_TOKEN_FILE", "PROGIT_TEST_TOKEN_ENV_VAR").unwrap();

        std::env::remove_var("PROGIT_TEST_TOKEN_FILE");

        assert_eq!(token.as_deref(), Some("abc123"));
    }
}
