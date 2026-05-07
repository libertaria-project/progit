// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2025 Markus Maiwald

//! Plugin runtime manager.
//!
//! Loads and manages plugins at runtime through the `progit-plugin-sdk`
//! (LSL-1.0). The TUI core never sees `mlua` types — that is the Trait
//! Firewall (Doctrine 4).
//!
//! [ARCH] Per-plugin failure isolation: a plugin that fails N consecutive
//! hooks is quarantined; subsequent dispatches skip it until the user
//! manually clears the quarantine. This prevents one broken plugin from
//! drowning the log and from re-entering broken state on every event.

use anyhow::{Context, Result};
use progit_plugin_sdk::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::highlight_cache::{key_for, HighlightCache};

/// After this many consecutive failed hook calls a plugin is quarantined.
const QUARANTINE_THRESHOLD: u32 = 5;

/// Plugin manager for ProGit.
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
    plugin_dir: PathBuf,
    /// Consecutive-error counter, keyed by plugin name. Reset on success.
    error_counts: HashMap<String, u32>,
    /// Plugin names currently quarantined, with the reason. Dispatch skips them.
    quarantined: HashMap<String, String>,
    /// Render-time highlight cache. Lives as long as the manager.
    highlight_cache: HighlightCache,
}

impl PluginManager {
    /// Create a new plugin manager.
    pub fn new(repo_root: &Path) -> Self {
        Self {
            plugins: Vec::new(),
            plugin_dir: repo_root.join("plugins"),
            error_counts: HashMap::new(),
            quarantined: HashMap::new(),
            highlight_cache: HighlightCache::new(),
        }
    }

    /// Load all plugins from the default plugins directory.
    pub fn load_all(&mut self, context: &PluginContext) -> Result<usize> {
        Ok(self.load_from_dir(&self.plugin_dir.clone(), context))
    }

    /// Load plugins from a specific directory.
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
            } else if path.is_dir() {
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

    /// Load a single plugin.
    ///
    /// If the plugin ships a `.progit-plugin.json` manifest next to its
    /// entry point, the manifest's capability block configures the runtime
    /// (network allowlist, memory cap, etc.). Otherwise legacy defaults apply.
    fn load_plugin(&mut self, path: &Path, context: &PluginContext) -> Result<()> {
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

        let mut plugin: Box<dyn Plugin> = match extension {
            "lua" => {
                let options = self.options_from_neighbouring_manifest(path, context);
                let lp = LuaPlugin::load_with_options(path, options)
                    .context(format!("Failed to load Lua plugin from {:?}", path))?;
                Box::new(lp)
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

    /// Look for a `.progit-plugin.json` next to the plugin entry-point and
    /// derive [`LuaPluginOptions`] from its capability block.
    ///
    /// Resolution order:
    /// 1. `<entry>/../.progit-plugin.json`  (directory-style plugin)
    /// 2. `<entry>.progit-plugin.json`      (file-style plugin, future use)
    fn options_from_neighbouring_manifest(
        &self,
        entry: &Path,
        context: &PluginContext,
    ) -> LuaPluginOptions {
        let candidates = [
            entry.parent().map(|p| p.join(".progit-plugin.json")),
            Some(entry.with_extension("progit-plugin.json")),
        ];

        for c in candidates.into_iter().flatten() {
            if !c.exists() {
                continue;
            }
            match PluginManifest::load(&c) {
                Ok(m) => {
                    if let Err(e) = m.check_sdk_compat(progit_plugin_sdk::SDK_API_VERSION) {
                        log::warn!("Plugin manifest {:?} rejected: {}", c, e);
                        continue;
                    }
                    if m.capabilities_implicit() {
                        log::warn!(
                            "Plugin '{}' did not declare capabilities — running with legacy defaults. \
                             Add a `capabilities` block to {:?} for forward compatibility.",
                            m.name,
                            c
                        );
                    }
                    let mut opts = LuaPluginOptions::from_capabilities(&m.effective_capabilities());
                    opts.repo_root = Some(PathBuf::from(&context.repo_path));
                    return opts;
                }
                Err(e) => {
                    log::warn!("Failed to parse plugin manifest {:?}: {}", c, e);
                }
            }
        }

        // No manifest — legacy defaults, with the repo root threaded through
        // so storage still works.
        let mut opts = LuaPluginOptions::default();
        opts.repo_root = Some(PathBuf::from(&context.repo_path));
        opts
    }

    /// Trigger on_issue_created hook for all plugins.
    pub fn on_issue_created(&mut self, issue: &Issue) {
        self.fire_hook(PluginHook::OnIssueCreated, issue);
    }

    /// Trigger on_issue_updated hook for all plugins.
    pub fn on_issue_updated(&mut self, issue: &Issue) {
        self.fire_hook(PluginHook::OnIssueUpdated, issue);
    }

    /// Trigger on_issue_deleted hook for all plugins.
    pub fn on_issue_deleted(&mut self, issue_id: &str) {
        let data = serde_json::json!({ "id": issue_id });
        self.fire_hook_raw(PluginHook::OnIssueDeleted, &data);
    }

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

    fn fire_hook_raw(&mut self, hook: PluginHook, data: &serde_json::Value) {
        // Two-phase loop so we don't hold &mut self.plugins while mutating
        // self.error_counts / self.quarantined inline. Collect (name, result)
        // first, then update bookkeeping.
        let mut outcomes: Vec<(String, std::result::Result<(), String>)> =
            Vec::with_capacity(self.plugins.len());

        for plugin in &mut self.plugins {
            let name = plugin.metadata().name.clone();
            if self.quarantined.contains_key(&name) {
                log::trace!("Skipping quarantined plugin '{}'", name);
                continue;
            }
            if !plugin.supports_hook(&hook) {
                continue;
            }
            let res = plugin
                .execute_hook(&hook, data)
                .map(|_| ())
                .map_err(|e| e.to_string());
            outcomes.push((name, res));
        }

        for (name, res) in outcomes {
            match res {
                Ok(()) => {
                    self.error_counts.remove(&name);
                }
                Err(e) => self.record_failure(&name, &hook_label(&hook), &e),
            }
        }
    }

    fn record_failure(&mut self, name: &str, what: &str, err: &str) {
        let count = self.error_counts.entry(name.to_string()).or_insert(0);
        *count += 1;
        log::warn!(
            "Plugin '{}' failed {} (consecutive errors: {}): {}",
            name,
            what,
            *count,
            err
        );
        if *count >= QUARANTINE_THRESHOLD {
            let reason = format!("{} consecutive failures; last error: {}", count, err);
            log::error!("Quarantining plugin '{}': {}", name, reason);
            self.quarantined.insert(name.to_string(), reason);
        }
    }

    /// Names of plugins that are currently quarantined, with the reason.
    pub fn quarantined_plugins(&self) -> impl Iterator<Item = (&str, &str)> {
        self.quarantined
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Manually clear quarantine for a plugin (e.g. after a config fix).
    /// Returns `true` if it was quarantined and is now cleared.
    pub fn unquarantine(&mut self, name: &str) -> bool {
        self.error_counts.remove(name);
        self.quarantined.remove(name).is_some()
    }

    /// Get list of loaded plugin names.
    pub fn loaded_plugins(&self) -> Vec<&str> {
        self.plugins
            .iter()
            .map(|p| p.metadata().name.as_str())
            .collect()
    }

    /// Get rich metadata for all loaded plugins.
    pub fn plugin_info(&self) -> Vec<&PluginMetadata> {
        self.plugins.iter().map(|p| p.metadata()).collect()
    }

    /// Get plugin count.
    pub fn count(&self) -> usize {
        self.plugins.len()
    }

    /// Dispatch a structured plugin event and collect responses.
    ///
    /// Quarantined plugins are skipped. A plugin that returns `Err` for an
    /// event still increments its error counter and may itself trigger a
    /// new quarantine after the threshold.
    pub fn dispatch_event(
        &mut self,
        event: &crate::plugins::PluginEvent,
    ) -> Result<Vec<serde_json::Value>> {
        let mut responses = Vec::new();
        let event_json =
            serde_json::to_value(event).context("Failed to serialize plugin event")?;

        let mut outcomes: Vec<(String, std::result::Result<Option<serde_json::Value>, String>)> =
            Vec::with_capacity(self.plugins.len());

        for plugin in &mut self.plugins {
            let name = plugin.metadata().name.clone();
            if self.quarantined.contains_key(&name) {
                continue;
            }
            let res = plugin
                .on_event(&event_json)
                .map_err(|e| e.to_string());
            outcomes.push((name, res));
        }

        for (name, res) in outcomes {
            match res {
                Ok(Some(r)) => {
                    self.error_counts.remove(&name);
                    log::debug!("Plugin '{}' responded to event", name);
                    responses.push(r);
                }
                Ok(None) => {
                    self.error_counts.remove(&name);
                }
                Err(e) => self.record_failure(&name, "event handler", &e),
            }
        }

        Ok(responses)
    }

    /// Cache-checked render-time highlight call.
    ///
    /// Iterates non-quarantined plugins, asks each `highlight()` until one
    /// returns `Some`, caches and returns the result. Returns `None` if no
    /// plugin handles the request — the host then falls through to plain
    /// text. A plugin error increments its consecutive-error counter.
    ///
    /// Hot path: the lookup is a single blake3-truncated u64 hash + a
    /// HashMap probe. Sub-microsecond on cache hit, which is the common case.
    pub fn highlight_cached(
        &mut self,
        language: Option<&str>,
        content: &str,
    ) -> Option<HighlightResponse> {
        if self.plugins.is_empty() {
            return None;
        }
        let key = key_for(language, content);
        if let Some(hit) = self.highlight_cache.get(key) {
            return Some(hit);
        }

        let request = HighlightRequest {
            language: language.map(str::to_string),
            content: content.to_string(),
        };

        // Two-phase: collect the first non-None outcome with bookkeeping.
        let mut chosen: Option<HighlightResponse> = None;
        let mut failures: Vec<(String, String)> = Vec::new();

        for plugin in &mut self.plugins {
            let name = plugin.metadata().name.clone();
            if self.quarantined.contains_key(&name) {
                continue;
            }
            match plugin.highlight(&request) {
                Ok(Some(resp)) => {
                    chosen = Some(resp);
                    self.error_counts.remove(&name);
                    break;
                }
                Ok(None) => {
                    self.error_counts.remove(&name);
                }
                Err(e) => failures.push((name, e.to_string())),
            }
        }

        for (name, err) in failures {
            self.record_failure(&name, "highlight", &err);
        }

        if let Some(ref resp) = chosen {
            self.highlight_cache.insert(key, resp.clone());
        }
        chosen
    }

    /// Drop the highlight cache. Call after a plugin reload or theme
    /// change so stale spans are not rendered.
    pub fn clear_highlight_cache(&mut self) {
        self.highlight_cache.clear();
    }

    /// Hit rate of the highlight cache, if any lookups have happened.
    pub fn highlight_cache_hit_rate(&self) -> Option<f64> {
        self.highlight_cache.hit_rate()
    }
}

fn hook_label(hook: &PluginHook) -> String {
    format!("{:?}", hook)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manager_creation() {
        let manager = PluginManager::new(Path::new("/tmp/test"));
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_unquarantine_returns_false_when_absent() {
        let mut m = PluginManager::new(Path::new("/tmp/test"));
        assert!(!m.unquarantine("never-existed"));
    }
}
