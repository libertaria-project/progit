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
    /// Legacy single-repo sync (backward compatible)
    pub sync: Option<SyncConfig>,
    
    /// Multi-repo configuration (new)
    pub repos: Vec<RepoConfig>,
    
    /// Theme preference
    pub theme: Option<String>,
    
    /// Web app settings (future commercial feature)
    pub web: Option<WebConfig>,
    
    /// Custom styles (key = component/name, value = StyleConfig)
    pub styles: std::collections::HashMap<String, StyleConfig>,
}

/// Style configuration
#[derive(Debug, Clone)]
pub struct StyleConfig {
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub modifiers: Vec<String>, // "bold", "italic", "dim", "underlined"
    pub inherits: Option<String>, // Name of style to inherit from
}

/// Repository configuration (multi-repo support)
#[derive(Debug, Clone)]
pub struct RepoConfig {
    /// Repository identifier (e.g., "frontend", "backend")
    pub name: String,
    
    /// Optional path to repository (relative or absolute)
    pub path: Option<String>,
    
    /// Sync configuration for this repo
    pub sync: SyncConfig,
}

/// Sync configuration (per-repo or global)
#[derive(Debug, Clone)]
pub struct SyncConfig {
    pub provider: String,  // "gitlab", "forgejo", "github"
    pub url: String,
    pub owner: String,
    pub repo: String,
}

/// Web app configuration (future commercial feature)
#[derive(Debug, Clone)]
pub struct WebConfig {
    /// Enable web interface
    pub enabled: bool,
    
    /// Port for web server
    pub port: u16,
    
    /// API token for web app authentication
    pub api_token: Option<String>,
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
    
    // Parse legacy single-repo sync (backward compatible)
    let sync = doc
        .nodes()
        .iter()
        .find(|n| n.name().value() == "sync")
        .map(parse_sync_node);
    
    // Parse multi-repo configuration
    let repos = doc
        .nodes()
        .iter()
        .find(|n| n.name().value() == "repos")
        .map(|n| parse_repos_node(n))
        .unwrap_or_default();

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
    
    // Parse styles
    let styles = doc
        .nodes()
        .iter()
        .find(|n| n.name().value() == "styles")
        .map(parse_styles_node)
        .unwrap_or_default();
    
    // Parse web config (future feature)
    let web = doc
        .nodes()
        .iter()
        .find(|n| n.name().value() == "web")
        .map(parse_web_node);

    Ok(Config { sync, repos, theme, web, styles })
}

fn parse_styles_node(node: &KdlNode) -> std::collections::HashMap<String, StyleConfig> {
    let mut styles = std::collections::HashMap::new();
    let children = node.children().map(|c| c.nodes()).unwrap_or(&[]);
    
    for child in children {
        if child.name().value() == "style" {
            if let Some(name) = child.entries().first().and_then(|e| e.value().as_string()) {
                let style_children = child.children().map(|c| c.nodes()).unwrap_or(&[]);
                
                let fg = get_string_value(style_children, "fg");
                let bg = get_string_value(style_children, "bg");
                
                // Handle inheritance
                let inherits = get_string_value(style_children, "inherits");
                
                // Collect modifiers
                let mut modifiers = Vec::new();
                for modifier_node in style_children {
                    let name = modifier_node.name().value();
                    if ["bold", "italic", "dim", "underlined", "reversed"].contains(&name) {
                         let is_true = modifier_node.entries().first()
                             .and_then(|e| e.value().as_bool())
                             .unwrap_or(true);
                        
                         if is_true {
                             modifiers.push(name.to_string());
                         }
                    }
                }
                
                styles.insert(name.to_string(), StyleConfig { fg, bg, modifiers, inherits });
            }
        }
    }
    
    styles
}

fn parse_repos_node(node: &KdlNode) -> Vec<RepoConfig> {
    let children = node.children().map(|c| c.nodes()).unwrap_or(&[]);
    
    children
        .iter()
        .filter(|n| n.name().value() == "repo")
        .filter_map(|n| {
            // Get repo name from first argument
            let name = n.entries().first()
                .and_then(|e| e.value().as_string())
                .map(|s| s.to_string())?;
            
            let repo_children = n.children().map(|c| c.nodes()).unwrap_or(&[]);
            
            // Parse path (optional)
            let path = get_string_value(repo_children, "path");
            
            // Parse sync config (required)
            let sync_node = repo_children.iter()
                .find(|n| n.name().value() == "sync")?;
            let sync = parse_sync_node(sync_node);
            
            Some(RepoConfig { name, path, sync })
        })
        .collect()
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

fn parse_web_node(node: &KdlNode) -> WebConfig {
    let children = node.children().map(|c| c.nodes()).unwrap_or(&[]);
    
    let enabled = get_bool_value(children, "enabled").unwrap_or(false);
    let port = get_int_value(children, "port")
        .and_then(|p| u16::try_from(p).ok())
        .unwrap_or(8080);
    let api_token = get_string_value(children, "api-token");
    
    WebConfig {
        enabled,
        port,
        api_token,
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

fn get_int_value(nodes: &[KdlNode], name: &str) -> Option<i64> {
    nodes
        .iter()
        .find(|n| n.name().value() == name)
        .and_then(|n| n.entries().first())
        .and_then(|e| e.value().as_i64())
}

fn get_bool_value(nodes: &[KdlNode], name: &str) -> Option<bool> {
    nodes
        .iter()
        .find(|n| n.name().value() == name)
        .and_then(|n| n.entries().first())
        .and_then(|e| e.value().as_bool())
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
    fn test_parse_legacy_config() {
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
        assert!(config.repos.is_empty());
    }
    
    #[test]
    fn test_parse_multi_repo_config() {
        let content = r#"
            repos {
                repo "frontend" {
                    path "../frontend"
                    sync {
                        provider "gitlab"
                        url "https://gitlab.dlabs.hu"
                        owner "markus.maiwald"
                        repo "my-frontend"
                    }
                }
                
                repo "backend" {
                    sync {
                        provider "forgejo"
                        url "https://git.example.com"
                        owner "markus"
                        repo "my-backend"
                    }
                }
            }
        "#;

        let config = parse_config(content).unwrap();
        assert_eq!(config.repos.len(), 2);
        
        let frontend = &config.repos[0];
        assert_eq!(frontend.name, "frontend");
        assert_eq!(frontend.path, Some("../frontend".to_string()));
        assert_eq!(frontend.sync.provider, "gitlab");
        
        let backend = &config.repos[1];
        assert_eq!(backend.name, "backend");
        assert!(backend.path.is_none());
        assert_eq!(backend.sync.provider, "forgejo");
    }
    
    #[test]
    fn test_parse_web_config() {
        let content = r#"
            web {
                enabled true
                port 3000
                api-token "secret123"
            }
        "#;

        let config = parse_config(content).unwrap();
        let web = config.web.unwrap();
        
        assert!(web.enabled);
        assert_eq!(web.port, 3000);
        assert_eq!(web.api_token, Some("secret123".to_string()));
    }
    
    #[test]
    fn test_parse_complete_config() {
        let content = r#"
            config {
                theme "vibe"
            }
            
            repos {
                repo "main" {
                    sync {
                        provider "gitlab"
                        url "https://gitlab.com"
                        owner "user"
                        repo "project"
                    }
                }
            }
            
            web {
                enabled false
                port 8080
            }
        "#;

        let config = parse_config(content).unwrap();
        assert_eq!(config.theme, Some("vibe".to_string()));
        assert_eq!(config.repos.len(), 1);
        assert!(config.web.is_some());
    }
}
