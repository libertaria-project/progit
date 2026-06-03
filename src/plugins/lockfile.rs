// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2025 Markus Maiwald

//! Plugin lockfile management
//!
//! Manages `.project/plugins.lock.kdl` for reproducible plugin installations.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use super::registry::PluginSource;

/// Information about a locked plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedPlugin {
    pub version: String,
    pub source: String,
    pub commit: Option<String>,
    pub sha256: Option<String>,
    pub installed: DateTime<Utc>,
}

/// Plugin lockfile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    /// When the lockfile was last updated
    pub locked_at: DateTime<Utc>,
    /// Locked plugins
    pub plugins: HashMap<String, LockedPlugin>,
}

impl Default for Lockfile {
    fn default() -> Self {
        Self::new()
    }
}

impl Lockfile {
    /// Create a new empty lockfile
    pub fn new() -> Self {
        Self {
            locked_at: Utc::now(),
            plugins: HashMap::new(),
        }
    }

    /// Load lockfile from path
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).context("Failed to read lockfile")?;

        // Try JSON first (simpler)
        if let Ok(lockfile) = serde_json::from_str::<Lockfile>(&content) {
            return Ok(lockfile);
        }

        // Parse KDL format
        Self::parse_kdl(&content)
    }

    /// Parse KDL lockfile format
    fn parse_kdl(content: &str) -> Result<Self> {
        let mut lockfile = Lockfile::new();

        // Parse locked timestamp
        if let Some(caps) = regex::Regex::new(r#"locked\s+"([^"]+)""#)
            .ok()
            .and_then(|re| re.captures(content))
        {
            if let Some(ts) = caps.get(1) {
                if let Ok(dt) = DateTime::parse_from_rfc3339(ts.as_str()) {
                    lockfile.locked_at = dt.with_timezone(&Utc);
                }
            }
        }

        // Parse plugin blocks
        // Simple regex-based parser for MVP
        let plugin_re = regex::Regex::new(
            r#"(?s)(\w[\w-]*)\s*\{\s*version\s+"([^"]+)"[^}]*source\s+"([^"]+)"[^}]*(?:commit\s+"([^"]+)")?[^}]*(?:sha256\s+"([^"]+)")?[^}]*(?:installed\s+"([^"]+)")?[^}]*\}"#
        ).ok();

        if let Some(re) = plugin_re {
            for caps in re.captures_iter(content) {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
                let version = caps.get(2).map(|m| m.as_str()).unwrap_or_default();
                let source = caps.get(3).map(|m| m.as_str()).unwrap_or_default();
                let commit = caps.get(4).map(|m| m.as_str().to_string());
                let sha256 = caps.get(5).map(|m| m.as_str().to_string());
                let installed = caps
                    .get(6)
                    .and_then(|m| DateTime::parse_from_rfc3339(m.as_str()).ok())
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(Utc::now);

                if !name.is_empty() && !version.is_empty() {
                    lockfile.plugins.insert(
                        name.to_string(),
                        LockedPlugin {
                            version: version.to_string(),
                            source: source.to_string(),
                            commit,
                            sha256,
                            installed,
                        },
                    );
                }
            }
        }

        Ok(lockfile)
    }

    /// Save lockfile to path
    pub fn save(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Generate KDL format
        let mut output = String::new();
        output.push_str("// Auto-generated plugin lockfile - do not edit manually\n");
        output.push_str("// Regenerate with: prog plugin lock\n\n");
        output.push_str(&format!("locked \"{}\"\n\n", self.locked_at.to_rfc3339()));
        output.push_str("plugins {\n");

        for (name, info) in &self.plugins {
            output.push_str(&format!("    {} {{\n", name));
            output.push_str(&format!("        version \"{}\"\n", info.version));
            output.push_str(&format!("        source \"{}\"\n", info.source));
            if let Some(commit) = &info.commit {
                output.push_str(&format!("        commit \"{}\"\n", commit));
            }
            if let Some(sha256) = &info.sha256 {
                output.push_str(&format!("        sha256 \"{}\"\n", sha256));
            }
            output.push_str(&format!(
                "        installed \"{}\"\n",
                info.installed.to_rfc3339()
            ));
            output.push_str("    }\n");
        }

        output.push_str("}\n");

        std::fs::write(path, output)?;

        Ok(())
    }

    /// Get version of a plugin
    pub fn get_version(&self, name: &str) -> Option<String> {
        self.plugins.get(name).map(|p| p.version.clone())
    }

    /// Add a plugin to the lockfile
    pub fn add_plugin(&mut self, name: &str, source: &PluginSource) -> Result<()> {
        let (version, source_url, commit) = match source {
            PluginSource::Registry { version, url, .. } => (version.clone(), url.clone(), None),
            PluginSource::Git { url, reference } => {
                let version = reference.clone().unwrap_or_else(|| "main".to_string());
                // Get actual commit SHA
                let commit = Self::get_git_commit(url, reference.as_deref())?;
                (version, url.clone(), commit)
            }
        };

        self.plugins.insert(
            name.to_string(),
            LockedPlugin {
                version,
                source: source_url,
                commit,
                sha256: None, // TODO: Calculate after download
                installed: Utc::now(),
            },
        );

        self.locked_at = Utc::now();

        Ok(())
    }

    /// Remove a plugin from the lockfile
    pub fn remove_plugin(&mut self, name: &str) {
        self.plugins.remove(name);
        self.locked_at = Utc::now();
    }

    /// Iterate over plugins
    pub fn plugins(&self) -> impl Iterator<Item = (&str, &LockedPlugin)> {
        self.plugins.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Get the commit SHA for a git reference
    fn get_git_commit(url: &str, reference: Option<&str>) -> Result<Option<String>> {
        let ref_str = reference.unwrap_or("HEAD");

        let output = std::process::Command::new("git")
            .args(["ls-remote", url, ref_str])
            .output()
            .context("Failed to run git ls-remote")?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = stdout.lines().next() {
                if let Some(commit) = line.split_whitespace().next() {
                    return Ok(Some(commit.to_string()));
                }
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lockfile_roundtrip() {
        let mut lockfile = Lockfile::new();
        lockfile.plugins.insert(
            "test-plugin".to_string(),
            LockedPlugin {
                version: "1.0.0".to_string(),
                source: "https://github.com/test/plugin.git".to_string(),
                commit: Some("abc123".to_string()),
                sha256: None,
                installed: Utc::now(),
            },
        );

        let temp_dir = std::env::temp_dir().join("progit-lockfile-test");
        let _ = std::fs::create_dir_all(&temp_dir);
        let path = temp_dir.join("plugins.lock.kdl");

        lockfile.save(&path).unwrap();

        let loaded = Lockfile::load(&path).unwrap();
        assert_eq!(loaded.plugins.len(), 1);
        assert!(loaded.plugins.contains_key("test-plugin"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
