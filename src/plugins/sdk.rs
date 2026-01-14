// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2025 Markus Maiwald
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0

//! ProGit Plugin SDK
//! 
//! [ARCH] Apache 2.0 licensed plugin SDK for community extensions.
//! Provides trait-based plugin system with LuaJIT runtime.
//! 
//! # Design Principles
//! 
//! 1. **Trait Firewall**: Plugin implementation details never leak to TUI core
//! 2. **JSON Boundary**: All data crossing plugin boundary is JSON
//! 3. **Runtime Agnostic**: LuaJIT today, WASM tomorrow, same SDK
//! 4. **Minimal Binary Impact**: Default build stays <7MB

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Plugin lifecycle events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum PluginEvent {
    /// ProGit started
    Startup,
    /// Issue created
    IssueCreated { issue_id: String },
    /// Issue updated
    IssueUpdated { issue_id: String },
    /// Issue status changed
    IssueStatusChanged { issue_id: String, old_status: String, new_status: String },
    /// Commit created
    CommitCreated { commit_hash: String },
    /// Virtual branch created
    BranchCreated { branch_id: String },
    /// Virtual branch updated
    BranchUpdated { branch_id: String },
    /// Agent action triggered
    AgentAction { action: String, branch_id: String },
    /// Custom event (for plugin-to-plugin communication)
    Custom { name: String, payload: serde_json::Value },
}

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: String,
}

/// Plugin SDK trait - implementation agnostic
pub trait Plugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;
    
    /// Called when plugin is loaded
    fn on_load(&mut self) -> Result<(), String>;
    
    /// Called when plugin is about to unload
    fn on_unload(&mut self) -> Result<(), String>;
    
    /// Handle an event
    /// 
    /// Returns optional JSON response for the event
    fn on_event(&mut self, event: &PluginEvent) -> Result<Option<serde_json::Value>, String>;
    
    /// Execute a plugin command
    /// 
    /// Commands can be triggered via `:plugin <name> <command> [args]`
    fn execute_command(&mut self, command: &str, args: &[String]) -> Result<String, String>;
}

/// Plugin engine trait - abstracts LuaJIT/WASM implementation
pub trait PluginEngine: Send + Sync {
    /// Load a plugin from source code
    fn load_plugin(&mut self, name: &str, source: &str) -> Result<(), String>;
    
    /// Unload a plugin
    fn unload_plugin(&mut self, name: &str) -> Result<(), String>;
    
    /// Send event to all loaded plugins
    fn dispatch_event(&mut self, event: &PluginEvent) -> Result<Vec<serde_json::Value>, String>;
    
    /// Execute command on specific plugin
    fn execute_command(&mut self, plugin_name: &str, command: &str, args: &[String]) -> Result<String, String>;
    
    /// List loaded plugins
    fn list_plugins(&self) -> Vec<PluginMetadata>;
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Plugin name
    pub name: String,
    /// Path to plugin file
    pub path: String,
    /// Whether plugin is enabled
    pub enabled: bool,
    /// Plugin-specific configuration
    pub config: HashMap<String, serde_json::Value>,
}

/// Plugin registry - manages installed plugins
pub struct PluginRegistry {
    plugins: Vec<PluginConfig>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }
    
    /// Load plugin registry from config
    pub fn load() -> Result<Self, String> {
        // TODO: Load from ~/.progit/plugins/config.json
        Ok(Self::new())
    }
    
    /// Save plugin registry to config
    pub fn save(&self) -> Result<(), String> {
        // TODO: Save to ~/.progit/plugins/config.json
        Ok(())
    }
    
    /// Add plugin to registry
    pub fn add(&mut self, config: PluginConfig) {
        self.plugins.push(config);
    }
    
    /// Remove plugin from registry
    pub fn remove(&mut self, name: &str) -> Result<(), String> {
        self.plugins.retain(|p| p.name != name);
        Ok(())
    }
    
    /// Get all enabled plugins
    pub fn enabled_plugins(&self) -> impl Iterator<Item = &PluginConfig> {
        self.plugins.iter().filter(|p| p.enabled)
    }
    
    /// Enable/disable plugin
    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> Result<(), String> {
        if let Some(plugin) = self.plugins.iter_mut().find(|p| p.name == name) {
            plugin.enabled = enabled;
            Ok(())
        } else {
            Err(format!("Plugin '{}' not found", name))
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_plugin_registry() {
        let mut registry = PluginRegistry::new();
        
        let config = PluginConfig {
            name: "test-plugin".to_string(),
            path: "/path/to/plugin.lua".to_string(),
            enabled: true,
            config: HashMap::new(),
        };
        
        registry.add(config);
        assert_eq!(registry.enabled_plugins().count(), 1);
        
        registry.set_enabled("test-plugin", false).unwrap();
        assert_eq!(registry.enabled_plugins().count(), 0);
    }
}
