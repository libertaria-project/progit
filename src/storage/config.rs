//! Configuration Storage
//!
//! Parses .projects/config.kdl

use anyhow::{Context, Result};
use kdl::{KdlDocument, KdlNode};
use std::fs;
use std::path::Path;

/// Project configuration
#[derive(Debug, Default, Clone)]
pub struct Config {
    pub sync: Option<SyncConfig>,
    pub theme: Option<String>,
}

/// Sync configuration
#[derive(Debug, Clone)]
pub struct SyncConfig {
    pub provider: String,
    pub url: String,
    pub owner: String,
    pub repo: String,
}

/// Load configuration from file
pub fn load_config(path: &Path) -> Result<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }

    let content = fs::read_to_string(path).context("Failed to read config file")?;
    parse_config(&content)
}

/// Parse config KDL content
pub fn parse_config(content: &str) -> Result<Config> {
    let doc: KdlDocument = content.parse().context("Failed to parse config KDL")?;
    
    let sync = doc
        .nodes()
        .iter()
        .find(|n| n.name().value() == "sync")
        .map(parse_sync_node);

    // Parse theme from config node
    let theme = doc
        .nodes()
        .iter()
        .find(|n| n.name().value() == "config")
        .and_then(|n| n.children())
        .and_then(|children| {
            children.nodes().iter()
                .find(|n| n.name().value() == "theme")
                .and_then(|n| n.entries().first())
                .and_then(|e| e.value().as_string())
                .map(|s| s.to_string())
        });

    Ok(Config { sync, theme })
}

fn parse_sync_node(node: &KdlNode) -> SyncConfig {
    let children = node.children().map(|c| c.nodes()).unwrap_or(&[]);
    
    let provider = get_string_value(children, "provider").unwrap_or_else(|| "forgejo".to_string());
    let url = get_string_value(children, "url").unwrap_or_default();
    let owner = get_string_value(children, "owner").unwrap_or_default();
    let repo = get_string_value(children, "repo").unwrap_or_default();

    SyncConfig {
        provider,
        url,
        owner,
        repo,
    }
}

// Helper to extract string values from KDL nodes
fn get_string_value(nodes: &[KdlNode], name: &str) -> Option<String> {
    nodes
        .iter()
        .find(|n| n.name().value() == name)
        .and_then(|n| n.entries().first())
        .and_then(|e| e.value().as_string())
        .map(|s| s.to_string())
}

/// Save theme to config file (updates theme line in-place)
pub fn save_theme(path: &Path, theme: &str) -> Result<()> {
    let content = if path.exists() {
        fs::read_to_string(path)?
    } else {
        return Ok(()); // No config file = nothing to save to
    };

    // Simple string-based update: find theme "..." and replace
    let new_content = if content.contains("theme \"") {
        // Replace existing theme line
        let re = regex::Regex::new(r#"theme\s+"[^"]*""#).unwrap();
        re.replace(&content, format!("theme \"{}\"", theme)).to_string()
    } else if content.contains("config {") {
        // Add theme to config block
        content.replace("config {", &format!("config {{\n    theme \"{}\"", theme))
    } else {
        // No config block - can't save
        return Ok(());
    };

    fs::write(path, new_content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let content = r#"
            sync {
                provider "forgejo"
                url "https://git.maiwald.work"
                owner "markus"
                repo "progit"
            }
        "#;

        let config = parse_config(content).unwrap();
        let sync = config.sync.unwrap();
        
        assert_eq!(sync.provider, "forgejo");
        assert_eq!(sync.url, "https://git.maiwald.work");
        assert_eq!(sync.owner, "markus");
        assert_eq!(sync.repo, "progit");
    }
}
