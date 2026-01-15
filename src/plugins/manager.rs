// SPDX-License-Identifier: EUPL-1.2
// Copyright (c) 2025 Markus Maiwald

//! Plugin runtime manager
//!
//! Loads and manages plugins at runtime.
//! Uses the Apache 2.0 licensed progit-plugin-sdk.
//!
//! ## Trait Firewall
//!
//! This module enforces the Trait Firewall doctrine: the TUI core only
//! interacts with plugins through the `Plugin` trait, never through
//! concrete runtime types like `LuaPlugin` or `WasmPlugin`.

use anyhow::{Context, Result};
use progit_plugin_sdk::prelude::*;
use std::path::{Path, PathBuf};

/// Plugin manager for ProGit
///
/// Uses trait objects (`Box<dyn Plugin>`) to maintain the Trait Firewall.
/// This allows swapping plugin runtimes without changing the TUI core.
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
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

    /// Load all plugins from the default plugins directory
    pub fn load_all(&mut self, context: &PluginContext) -> Result<usize> {
        Ok(self.load_from_dir(&self.plugin_dir.clone(), context))
    }

    /// Load plugins from a specific directory
    pub fn load_from_dir(&mut self, dir: &Path, context: &PluginContext) -> usize {
        if !dir.exists() {
            log::info!("No plugins directory found at {:?}", dir);
            return 0;
        }

        let mut loaded = 0;

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                log::warn!("Failed to read plugins directory {:?}: {}", dir, e);
                return 0;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();

            // Load .lua files directly
            if path.extension().and_then(|s| s.to_str()) == Some("lua") {
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
            // Load plugins from directories (for installed plugins)
            else if path.is_dir() {
                // Look for main.lua or src/main.lua
                let main_lua = path.join("main.lua");
                let src_main_lua = path.join("src").join("main.lua");

                let entry_point = if main_lua.exists() {
                    Some(main_lua)
                } else if src_main_lua.exists() {
                    Some(src_main_lua)
                } else {
                    None
                };

                if let Some(lua_path) = entry_point {
                    match self.load_plugin(&lua_path, context) {
                        Ok(()) => {
                            loaded += 1;
                            log::info!("Loaded plugin from directory: {}", path.display());
                        }
                        Err(e) => {
                            log::warn!("Failed to load plugin {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }

        loaded
    }

    /// Load a single plugin
    ///
    /// Detects plugin type by file extension and loads via appropriate runtime.
    fn load_plugin(&mut self, path: &Path, context: &PluginContext) -> Result<()> {
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

        let mut plugin: Box<dyn Plugin> = match extension {
            "lua" => {
                Box::new(LuaPlugin::load(path).context(format!(
                    "Failed to load Lua plugin from {:?}",
                    path
                ))?)
            }
            // WASM support can be added here when ready:
            // "wasm" => Box::new(WasmPlugin::load(path)?),
            _ => {
                anyhow::bail!("Unknown plugin extension: {}", extension);
            }
        };

        plugin
            .init(context)
            .context("Failed to initialize plugin")?;

        self.plugins.push(plugin);
        Ok(())
    }

    /// Trigger on_issue_created hook for all plugins
    pub fn on_issue_created(&mut self, issue: &Issue) {
        self.fire_hook(PluginHook::OnIssueCreated, issue);
    }

    /// Trigger on_issue_updated hook for all plugins
    pub fn on_issue_updated(&mut self, issue: &Issue) {
        self.fire_hook(PluginHook::OnIssueUpdated, issue);
    }

    /// Trigger on_issue_deleted hook for all plugins
    pub fn on_issue_deleted(&mut self, issue_id: &str) {
        let data = serde_json::json!({ "id": issue_id });
        self.fire_hook_raw(PluginHook::OnIssueDeleted, &data);
    }

    /// Fire a hook with serializable data
    fn fire_hook<T: serde::Serialize>(&mut self, hook: PluginHook, data: &T) {
        let json_data = match serde_json::to_value(data) {
            Ok(v) => v,
            Err(e) => {
                log::error!("Failed to serialize hook data: {}", e);
                return;
            }
        };
        self.fire_hook_raw(hook, &json_data);
    }

    /// Fire a hook with raw JSON data
    fn fire_hook_raw(&mut self, hook: PluginHook, data: &serde_json::Value) {
        for plugin in &mut self.plugins {
            if !plugin.supports_hook(&hook) {
                continue;
            }
            if let Err(e) = plugin.execute_hook(&hook, data) {
                log::warn!(
                    "Plugin '{}' failed {:?}: {}",
                    plugin.metadata().name,
                    hook,
                    e
                );
            }
        }
    }

    /// Get list of loaded plugins
    pub fn loaded_plugins(&self) -> Vec<&str> {
        self.plugins
            .iter()
            .map(|p| p.metadata().name.as_str())
            .collect()
    }

    /// Get plugin count
    pub fn count(&self) -> usize {
        self.plugins.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manager_creation() {
        let manager = PluginManager::new(Path::new("/tmp/test"));
        assert_eq!(manager.count(), 0);
    }
}
