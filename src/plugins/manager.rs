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
            return 0;
        }

        let mut loaded = 0;
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_err) => {
                return 0;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("lua") {
                match self.load_plugin(&path, context) {
                    Ok(()) => {
                        loaded += 1;
                    }
                    Err(_err) => {}
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
                        }
                        Err(_err) => {}
                    }
                } else {
                }
            }
        }

        loaded
    }

    /// Load only plugins that may handle a command namespace.
    ///
    /// If a plugin manifest declares `commands`, this filters by that list. If
    /// it declares only the legacy `hooks = ["on_command"]`, the plugin is
    /// loaded and can decide by returning `handled = false`.
    pub fn load_command_plugins_from_dir(
        &mut self,
        dir: &Path,
        context: &PluginContext,
        command: &str,
    ) -> usize {
        if !dir.exists() {
            return 0;
        }

        let mut loaded = 0;
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return 0,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let entry_point = if path.extension().and_then(|s| s.to_str()) == Some("lua") {
                Some(path.clone())
            } else if path.is_dir() {
                let main_lua = path.join("main.lua");
                let src_main_lua = path.join("src").join("main.lua");
                if main_lua.exists() {
                    Some(main_lua)
                } else if src_main_lua.exists() {
                    Some(src_main_lua)
                } else {
                    None
                }
            } else {
                None
            };

            let Some(lua_path) = entry_point else {
                continue;
            };
            if !command_manifest_allows(&lua_path, command) {
                continue;
            }
            if self.load_plugin(&lua_path, context).is_ok() {
                loaded += 1;
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

        if extension != "lua" {
            anyhow::bail!("Unsupported plugin extension for {:?}: {}", path, extension);
        }

        // Load plugin with panic isolation
        let options = self.options_from_neighbouring_manifest(path, context);
        let load_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            LuaPlugin::load_with_options(path, options)
        }));

        let plugin: Box<dyn Plugin> = match load_result {
            Ok(Ok(lp)) => {
                Box::new(lp)
            }
            Ok(Err(e)) => {
                anyhow::bail!("Failed to load Lua plugin from {:?}: {}", path, e);
            }
            Err(panic_info) => {
                let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic".to_string()
                };
                anyhow::bail!("Plugin loading panicked: {}", msg);
            }
        };

        // Use Box::into_raw to prevent automatic drop on panic
        let plugin_ptr = Box::into_raw(plugin);

        let init_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // SAFETY: plugin_ptr is valid, we own it
            unsafe { (*plugin_ptr).init(context) }
        }));

        match init_result {
            Ok(Ok(())) => {
                // Re-box the raw pointer and push
                self.plugins.push(unsafe { Box::from_raw(plugin_ptr) });
            }
            Ok(Err(e)) => {
                // SAFETY: Init failed, drop the plugin
                unsafe { drop(Box::from_raw(plugin_ptr)) };
                return Err(anyhow::anyhow!("Failed to initialize plugin: {}", e));
            }
            Err(panic_info) => {
                let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic".to_string()
                };
                // SAFETY: Init panicked, drop the plugin
                unsafe { drop(Box::from_raw(plugin_ptr)) };
                return Err(anyhow::anyhow!("Plugin init panicked: {}", msg));
            }
        }

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
                    let caps = m.effective_capabilities();
                    let repo_root = PathBuf::from(&context.repo_path);
                    let mut opts = LuaPluginOptions::from_capabilities(&caps);
                    opts.repo_root = Some(PathBuf::from(&context.repo_path));
                    if caps.sober {
                        opts.sober = Some(crate::sober::host_capability(repo_root));
                    }
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

    /// Dispatch a custom command to plugins that support the OnCommand hook.
    ///
    /// Plugins return `Some(result)` if they handle the command, `None` otherwise.
    /// The first plugin that handles the command "wins" - subsequent plugins are
    /// not consulted. This allows one plugin to own a command namespace.
    pub fn dispatch_command(
        &mut self,
        command: &str,
        args: &[String],
    ) -> Option<CommandResult> {
        let hook = PluginHook::OnCommand(command.to_string());
        let data = serde_json::json!({
            "command": command,
            "args": args
        });

        // Two-phase: collect result first, then update bookkeeping
        let mut pending_failure: Option<(String, String)> = None;

        for plugin in &mut self.plugins {
            let name = plugin.metadata().name.clone();

            if self.quarantined.contains_key(&name) {
                continue;
            }
            let supports = plugin.supports_hook(&hook);

            if !supports {
                continue;
            }

            let result = plugin.execute_hook(&hook, &data);

            match result {
                Ok(response) => {
                    let handled = response
                        .get("handled")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    if !handled {
                        self.error_counts.remove(&name);
                        continue;
                    }
                    self.error_counts.remove(&name);
                    return Some(CommandResult {
                        plugin: name,
                        success: response.get("success").and_then(|v| v.as_bool()).unwrap_or(true),
                        output: response.get("output").and_then(|v| v.as_str()).map(String::from),
                        error: response.get("error").and_then(|v| v.as_str()).map(String::from),
                        data: response,
                    });
                }
                Err(e) => {
                    pending_failure = Some((name, e.to_string()));
                }
            }
        }

        if let Some((name, err)) = pending_failure {
            self.record_failure(&name, &format!("command:{}", command), &err);
        }

        None
    }
}

fn hook_label(hook: &PluginHook) -> String {
    format!("{:?}", hook)
}

fn command_manifest_allows(entry: &Path, command: &str) -> bool {
    let candidates = [
        entry.parent().map(|p| p.join(".progit-plugin.json")),
        Some(entry.with_extension("progit-plugin.json")),
    ];

    for candidate in candidates.into_iter().flatten() {
        if !candidate.exists() {
            continue;
        }

        let Ok(raw) = std::fs::read_to_string(&candidate) else {
            return true;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
            return true;
        };

        if let Some(commands) = value.get("commands").and_then(|v| v.as_array()) {
            return commands
                .iter()
                .any(|item| command_entry_matches(item, command));
        }

        return value
            .get("hooks")
            .and_then(|v| v.as_array())
            .map(|hooks| hooks.iter().any(|h| h.as_str() == Some("on_command")))
            .unwrap_or(false);
    }

    true
}

fn command_entry_matches(item: &serde_json::Value, command: &str) -> bool {
    if item.as_str() == Some(command) {
        return true;
    }

    item.as_object().is_some_and(|object| {
        object
            .get("name")
            .and_then(|v| v.as_str())
            .is_some_and(|name| name == command)
            || object
                .get("alias")
                .and_then(|v| v.as_str())
                .is_some_and(|alias| alias == command)
    })
}

/// Result of a plugin command execution
#[derive(Debug)]
pub struct CommandResult {
    pub plugin: String,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub data: serde_json::Value,
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

    #[test]
    fn record_failure_thresholds_into_quarantine_then_unquarantines() {
        let mut m = PluginManager::new(Path::new("/tmp/test"));
        let name = "noisy";

        // Below the threshold → not quarantined yet.
        for _ in 0..(QUARANTINE_THRESHOLD - 1) {
            m.record_failure(name, "hook", "boom");
        }
        assert!(m.quarantined_plugins().count() == 0);

        // One more failure crosses the threshold.
        m.record_failure(name, "hook", "boom");
        let quarantined: Vec<_> = m.quarantined_plugins().collect();
        assert_eq!(quarantined.len(), 1);
        assert_eq!(quarantined[0].0, name);
        assert!(quarantined[0].1.contains("boom"));

        // Manual unquarantine clears state and resets the counter.
        assert!(m.unquarantine(name));
        assert_eq!(m.quarantined_plugins().count(), 0);

        // After clearing, failures restart from zero — old strikes don't carry.
        m.record_failure(name, "hook", "fresh");
        assert_eq!(m.quarantined_plugins().count(), 0);
    }

    #[test]
    fn loads_sober_raccoon_premium_plugin() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
        let plugin_dir = repo.join("plugins").join("sober-raccoon");
        let context = PluginContext {
            repo_path: repo.to_string_lossy().to_string(),
            user: None,
            env: Default::default(),
            config: Default::default(),
        };
        let mut manager = PluginManager::new(repo);

        let loaded = manager.load_from_dir(&plugin_dir, &context);

        assert_eq!(loaded, 1);
        assert_eq!(manager.loaded_plugins(), vec!["sober-raccoon"]);
    }

    #[test]
    fn sober_raccoon_lists_command_routes() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
        let plugin_dir = repo.join("plugins").join("sober-raccoon");
        let context = PluginContext {
            repo_path: repo.to_string_lossy().to_string(),
            user: None,
            env: Default::default(),
            config: Default::default(),
        };
        let mut manager = PluginManager::new(repo);

        manager.load_from_dir(&plugin_dir, &context);
        let result = manager
            .dispatch_command("sober-raccoon", &["route".to_string(), "list".to_string()])
            .expect("sober-raccoon should handle route list");

        assert!(result.success);
        let output = result.output.expect("route list should return output");
        assert!(output.contains("Sober Raccoon routes"));
        assert!(output.contains("prog plugin sober <args...>"));
    }

    #[test]
    fn dispatch_command_skips_unhandled_responses() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first.lua");
        let second = dir.path().join("second.lua");
        std::fs::write(
            &first,
            r#"
plugin = {
    name = "first",
    version = "1",
    author = "test",
    hooks = { on_command = true },
}

function on_command(_data)
    return { handled = false, success = false, error = "not mine" }
end
"#,
        )
        .unwrap();
        std::fs::write(
            &second,
            r#"
plugin = {
    name = "second",
    version = "1",
    author = "test",
    hooks = { on_command = true },
}

function on_command(data)
    return {
        handled = true,
        success = true,
        output = "handled " .. data.command,
    }
end
"#,
        )
        .unwrap();

        let context = PluginContext {
            repo_path: dir.path().to_string_lossy().to_string(),
            user: None,
            env: Default::default(),
            config: Default::default(),
        };
        let mut manager = PluginManager::new(dir.path());

        manager.load_plugin(&first, &context).unwrap();
        manager.load_plugin(&second, &context).unwrap();
        let result = manager
            .dispatch_command("sober", &[])
            .expect("second plugin should handle command");

        assert_eq!(result.plugin, "second");
        assert_eq!(result.output.as_deref(), Some("handled sober"));
    }

    #[test]
    fn command_loader_uses_manifest_command_names() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
        let context = PluginContext {
            repo_path: repo.to_string_lossy().to_string(),
            user: None,
            env: Default::default(),
            config: Default::default(),
        };
        let mut manager = PluginManager::new(repo);

        let loaded = manager.load_command_plugins_from_dir(&repo.join("plugins"), &context, "hooks");
        let plugins = manager.loaded_plugins();

        assert!(loaded > 0);
        assert!(plugins.iter().any(|name| *name == "git-hooks"));
        assert!(!plugins.iter().any(|name| *name == "gitlab-ci"));
    }
}
