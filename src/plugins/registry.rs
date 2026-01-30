// SPDX-License-Identifier: EUPL-1.2
// Copyright (c) 2025 Markus Maiwald

//! Plugin registry management
//!
//! Handles fetching and caching the plugin index from the git-based registry.
//! Registry model: Index repo + Plugin repos (Homebrew-style)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Default registry URL
const DEFAULT_REGISTRY_URL: &str = "https://github.com/progit-plugins/index.git";

/// Detect git origin remote URL from project root
fn detect_git_origin(project_root: &Path) -> Option<String> {
    let git_dir = project_root.join(".git");
    if !git_dir.exists() {
        return None;
    }

    // Try to read .git/config
    let config_path = git_dir.join("config");
    if let Ok(content) = std::fs::read_to_string(config_path) {
        // Simple regex to find origin URL
        if let Ok(re) = regex::Regex::new(r#"url\s*=\s*([^\n]+)"#) {
            if let Some(caps) = re.captures(&content) {
                if let Some(url) = caps.get(1) {
                    return Some(url.as_str().trim().to_string());
                }
            }
        }
    }

    None
}

/// Plugin manifest from registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub plugin_type: String,
    pub runtime: String,
    pub source_url: String,
    pub source_tag: Option<String>,
    pub source_commit: Option<String>,
    pub sdk_version: String,
    pub sha256: Option<String>,
}

/// Source from which to install a plugin
#[derive(Debug, Clone)]
pub enum PluginSource {
    /// Install from registry
    Registry {
        name: String,
        version: String,
        url: String,
    },
    /// Install directly from git URL
    Git {
        url: String,
        reference: Option<String>,
    },
}

impl PluginSource {
    /// Install the plugin to the given directory
    pub fn install(&self, plugin_dir: &Path) -> Result<PathBuf> {
        match self {
            PluginSource::Registry { name, version, url } => {
                let target_dir = plugin_dir.join(name);

                // Remove existing if present
                if target_dir.exists() {
                    std::fs::remove_dir_all(&target_dir)?;
                }

                // Shallow clone with specific tag/version
                let tag = format!("v{}", version);
                let status = Command::new("git")
                    .args(["clone", "--depth", "1", "--branch", &tag, url, target_dir.to_str().unwrap()])
                    .status()
                    .context("Failed to run git clone")?;

                if !status.success() {
                    // Try without tag (maybe it's a branch or commit)
                    let status = Command::new("git")
                        .args(["clone", "--depth", "1", url, target_dir.to_str().unwrap()])
                        .status()
                        .context("Failed to run git clone")?;

                    if !status.success() {
                        anyhow::bail!("Git clone failed for {}", url);
                    }
                }

                // Remove .git directory to save space
                let git_dir = target_dir.join(".git");
                if git_dir.exists() {
                    std::fs::remove_dir_all(&git_dir)?;
                }

                Ok(target_dir)
            }
            PluginSource::Git { url, reference } => {
                // Extract repo name from URL
                let name = url
                    .rsplit('/')
                    .next()
                    .unwrap_or("plugin")
                    .trim_end_matches(".git");

                let target_dir = plugin_dir.join(name);

                // Remove existing if present
                if target_dir.exists() {
                    std::fs::remove_dir_all(&target_dir)?;
                }

                // Clone with optional reference
                let mut args = vec!["clone", "--depth", "1"];
                if let Some(ref_str) = reference {
                    args.push("--branch");
                    args.push(ref_str);
                }
                args.push(url);
                args.push(target_dir.to_str().unwrap());

                let status = Command::new("git")
                    .args(&args)
                    .status()
                    .context("Failed to run git clone")?;

                if !status.success() {
                    anyhow::bail!("Git clone failed for {}", url);
                }

                // Remove .git directory
                let git_dir = target_dir.join(".git");
                if git_dir.exists() {
                    std::fs::remove_dir_all(&git_dir)?;
                }

                Ok(target_dir)
            }
        }
    }
}

/// Plugin registry client
pub struct PluginRegistry {
    /// Path to cached index
    index_path: PathBuf,
    /// Registry URL
    registry_url: String,
    /// Cached manifests
    manifests: HashMap<String, PluginManifest>,
}

impl PluginRegistry {
    /// Create a new registry client
    ///
    /// Registry URL resolution order (first match wins):
    /// 1. PROGIT_PLUGIN_REGISTRY environment variable (for testing)
    /// 2. Config-provided registry_url (.project/config.kdl)
    /// 3. Git origin remote URL (if available)
    /// 4. Default hardcoded registry URL
    pub fn new(project_root: &Path, config_registry_url: Option<String>) -> Result<Self> {
        let index_path = project_root
            .join(".progit")
            .join("plugin-index");

        // Determine registry URL with fallback chain
        let registry_url = std::env::var("PROGIT_PLUGIN_REGISTRY")
            .ok()
            .or(config_registry_url)
            .or_else(|| detect_git_origin(project_root))
            .unwrap_or_else(|| DEFAULT_REGISTRY_URL.to_string());

        let mut registry = Self {
            index_path,
            registry_url,
            manifests: HashMap::new(),
        };

        // Load cached index if available
        registry.load_cached_index()?;

        Ok(registry)
    }

    /// Update the index from remote
    pub fn update_index(&self) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.index_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if self.index_path.exists() {
            // Pull updates
            let status = Command::new("git")
                .args(["pull", "--ff-only"])
                .current_dir(&self.index_path)
                .status()
                .context("Failed to run git pull")?;

            if !status.success() {
                // Reset and pull fresh
                let _ = Command::new("git")
                    .args(["reset", "--hard", "origin/main"])
                    .current_dir(&self.index_path)
                    .status();
            }
        } else {
            // Fresh clone
            let status = Command::new("git")
                .args([
                    "clone",
                    "--depth",
                    "1",
                    &self.registry_url,
                    self.index_path.to_str().unwrap(),
                ])
                .status()
                .context("Failed to clone plugin index")?;

            if !status.success() {
                anyhow::bail!("Failed to clone plugin registry from {}", self.registry_url);
            }
        }

        Ok(())
    }

    /// Load cached index manifests
    fn load_cached_index(&mut self) -> Result<()> {
        let plugins_dir = self.index_path.join("plugins");

        if !plugins_dir.exists() {
            // No cached index yet - that's fine
            return Ok(());
        }

        // Recursively find all .kdl or .json manifest files
        self.load_manifests_from_dir(&plugins_dir)?;

        Ok(())
    }

    /// Recursively load manifests from directory
    fn load_manifests_from_dir(&mut self, dir: &Path) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                self.load_manifests_from_dir(&path)?;
            } else if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&content) {
                        self.manifests.insert(manifest.name.clone(), manifest);
                    }
                }
            } else if path.extension().map(|e| e == "kdl").unwrap_or(false) {
                // Parse KDL manifest (simplified - just extract JSON-like structure)
                if let Ok(manifest) = self.parse_kdl_manifest(&path) {
                    self.manifests.insert(manifest.name.clone(), manifest);
                }
            }
        }

        Ok(())
    }

    /// Parse a KDL manifest file (simplified parser)
    fn parse_kdl_manifest(&self, path: &Path) -> Result<PluginManifest> {
        let content = std::fs::read_to_string(path)?;

        // Simple regex-based extraction for MVP
        // In production, use a proper KDL parser
        let extract = |key: &str| -> String {
            let pattern = format!(r#"{}\s+"([^"]+)""#, key);
            regex::Regex::new(&pattern)
                .ok()
                .and_then(|re| re.captures(&content))
                .and_then(|caps| caps.get(1))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default()
        };

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(PluginManifest {
            name: name.clone(),
            version: extract("version"),
            description: extract("description"),
            author: extract("author"),
            license: extract("license"),
            plugin_type: extract("type"),
            runtime: extract("runtime"),
            source_url: extract("url"),
            source_tag: Some(extract("tag")).filter(|s| !s.is_empty()),
            source_commit: Some(extract("commit")).filter(|s| !s.is_empty()),
            sdk_version: extract("sdk_version"),
            sha256: Some(extract("sha256")).filter(|s| !s.is_empty()),
        })
    }

    /// Find a plugin by name
    pub fn find_plugin(&self, name: &str) -> Result<Option<PluginManifest>> {
        // Check cache first
        if let Some(manifest) = self.manifests.get(name) {
            return Ok(Some(manifest.clone()));
        }

        // Try updating index if not found
        if !self.index_path.exists() {
            self.update_index()?;

            // Reload manifests
            let plugins_dir = self.index_path.join("plugins");
            if plugins_dir.exists() {
                // Create a mutable copy to load
                let mut manifests = HashMap::new();
                Self::load_manifests_recursive(&plugins_dir, &mut manifests)?;

                if let Some(manifest) = manifests.get(name) {
                    return Ok(Some(manifest.clone()));
                }
            }
        }

        Ok(None)
    }

    /// Helper to load manifests recursively into a map
    fn load_manifests_recursive(dir: &Path, manifests: &mut HashMap<String, PluginManifest>) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                Self::load_manifests_recursive(&path, manifests)?;
            } else if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&content) {
                        manifests.insert(manifest.name.clone(), manifest);
                    }
                }
            }
        }

        Ok(())
    }

    /// Search plugins by query
    pub fn search(&self, query: &str) -> Result<Vec<PluginManifest>> {
        let query_lower = query.to_lowercase();

        let results: Vec<_> = self.manifests.values()
            .filter(|m| {
                m.name.to_lowercase().contains(&query_lower) ||
                m.description.to_lowercase().contains(&query_lower) ||
                m.plugin_type.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect();

        Ok(results)
    }
}
