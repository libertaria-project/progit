// SPDX-License-Identifier: EUPL-1.2
// Copyright (c) 2025 Markus Maiwald

//! Plugin management system
//!
//! Loads and manages Lua plugins from the plugins/ directory.
//! Uses the Apache 2.0 licensed progit-plugin-sdk.

use anyhow::{Context, Result};
use progit_plugin_sdk::prelude::*;
use std::path::{Path, PathBuf};

/// Plugin manager for ProGit
pub struct PluginManager {
    plugins: Vec<LuaPlugin>,
    plugin_dir: PathBuf,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(repo_root: &Path) -> Self {
        Self {
            plugins: Vec::new(),
            plugin_dir: repo_root.join("plugins"),
        }
    }
    
    /// Load all plugins from the plugins directory
    pub fn load_all(&mut self, context: &PluginContext) -> Result<usize> {
        if !self.plugin_dir.exists() {
            log::info!("No plugins directory found at {:?}", self.plugin_dir);
            return Ok(0);
        }
        
        let mut loaded = 0;
        
        for entry in std::fs::read_dir(&self.plugin_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            // Only load .lua files
            if path.extension().and_then(|s| s.to_str()) != Some("lua") {
                continue;
            }
            
            match self.load_plugin(&path, context) {
                Ok(()) => {
                    loaded += 1;
                    log::info!("Loaded plugin: {}", path.display());
                }
                Err(e) => {
                    log::warn!("Failed to load plugin {}: {}", path.display(), e);
                }
            }
        }
        
        Ok(loaded)
    }
    
    /// Load a single plugin
    fn load_plugin(&mut self, path: &Path, context: &PluginContext) -> Result<()> {
        let mut plugin = LuaPlugin::load(path)
            .context(format!("Failed to load plugin from {:?}", path))?;
        
        plugin.init(context)
            .context("Failed to initialize plugin")?;
        
        self.plugins.push(plugin);
        Ok(())
    }
    
    /// Trigger on_issue_created hook for all plugins
    pub fn on_issue_created(&mut self, issue: &Issue) {
        for plugin in &mut self.plugins {
            if let Err(e) = plugin.on_issue_created(issue) {
                log::warn!(
                    "Plugin '{}' failed on_issue_created: {}",
                    plugin.metadata().name,
                    e
                );
            }
        }
    }
    
    /// Trigger on_issue_updated hook for all plugins
    pub fn on_issue_updated(&mut self, issue: &Issue) {
        for plugin in &mut self.plugins {
            if let Err(e) = plugin.on_issue_updated(issue) {
                log::warn!(
                    "Plugin '{}' failed on_issue_updated: {}",
                    plugin.metadata().name,
                    e
                );
            }
        }
    }
    
    /// Trigger on_issue_deleted hook for all plugins
    pub fn on_issue_deleted(&mut self, issue_id: &str) {
        for plugin in &mut self.plugins {
            if let Err(e) = plugin.on_issue_deleted(issue_id) {
                log::warn!(
                    "Plugin '{}' failed on_issue_deleted: {}",
                    plugin.metadata().name,
                    e
                );
            }
        }
    }
    
    /// Get list of loaded plugins
    pub fn loaded_plugins(&self) -> Vec<&str> {
        self.plugins.iter().map(|p| p.metadata().name.as_str()).collect()
    }
    
    /// Get plugin count
    pub fn count(&self) -> usize {
        self.plugins.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_plugin_manager_creation() {
        let manager = PluginManager::new(Path::new("/tmp/test"));
        assert_eq!(manager.count(), 0);
    }
}
