// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2025 Markus Maiwald

//! Plugin CLI commands
//!
//! Implements the `prog plugin` subcommand for managing plugins:
//! - list: Show installed plugins
//! - install: Install a plugin from registry or git URL
//! - remove: Uninstall a plugin
//! - update: Update plugins
//! - search: Search the plugin registry
//! - info: Show plugin details

use anyhow::{Context, Result};
use colored::*;
use std::path::{Path, PathBuf};

use super::lockfile::Lockfile;
use super::manager::CommandResult;
use super::registry::{PluginRegistry, PluginSource};
use crate::storage::config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PluginInstallScope {
    Project,
    User,
}

impl PluginInstallScope {
    fn label(&self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::User => "user",
        }
    }
}

#[derive(Debug)]
struct DiscoveredPlugin {
    name: String,
    version: String,
    scope: PluginInstallScope,
}

fn plugin_dirs(project_root: &Path) -> Vec<(PathBuf, PluginInstallScope)> {
    vec![
        (project_root.join("plugins"), PluginInstallScope::Project),
        (
            project_root.join(".progit").join("plugins"),
            PluginInstallScope::User,
        ),
    ]
}

fn plugin_from_file_entry(
    path: &Path,
    lockfile: Option<&Lockfile>,
    scope: PluginInstallScope,
) -> DiscoveredPlugin {
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    let version = lockfile
        .and_then(|lf| lf.get_version(name))
        .unwrap_or_else(|| "local".to_string());

    DiscoveredPlugin {
        name: name.to_string(),
        version,
        scope,
    }
}

fn collect_installed_plugins(project_root: &Path) -> Result<Vec<DiscoveredPlugin>> {
    let lockfile_path = project_root.join(".project").join("plugins.lock.kdl");
    let lockfile = if lockfile_path.exists() {
        Lockfile::load(&lockfile_path).ok()
    } else {
        None
    };

    let mut plugins = Vec::new();

    for (plugin_dir, scope) in plugin_dirs(project_root) {
        if !plugin_dir.exists() {
            continue;
        }

        for entry in std::fs::read_dir(&plugin_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "lua").unwrap_or(false) {
                plugins.push(plugin_from_file_entry(&path, lockfile.as_ref(), scope));
                continue;
            }

            if path.is_dir() {
                let manifest_path = path.join(".progit-plugin.json");
                if manifest_path.exists() {
                    let name = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown");

                    let version = lockfile
                        .as_ref()
                        .and_then(|lf| lf.get_version(name))
                        .unwrap_or_else(|| "local".to_string());

                    plugins.push(DiscoveredPlugin {
                        name: name.to_string(),
                        version,
                        scope,
                    });
                }
            }
        }
    }

    plugins.sort_by(|a, b| {
        (a.scope.label(), a.name.as_str()).cmp(&(b.scope.label(), b.name.as_str()))
    });

    Ok(plugins)
}

fn plugin_dirs_exist(project_root: &Path) -> bool {
    plugin_dirs(project_root)
        .into_iter()
        .any(|(path, _)| path.exists())
}

fn remove_plugin_candidates(project_root: &Path, name: &str) -> Vec<PathBuf> {
    plugin_dirs(project_root)
        .into_iter()
        .flat_map(|(plugin_dir, _)| {
            let lua_path = plugin_dir.join(format!("{}.lua", name));
            let dir_path = plugin_dir.join(name);

            std::iter::once(lua_path)
                .chain(std::iter::once(dir_path))
                .filter(|path| path.exists())
                .collect::<Vec<_>>()
                .into_iter()
        })
        .collect()
}

/// Convert a TUI `:plugin ...` command into a `prog plugin ...` process.
pub fn tui_command_args(parts: &[&str]) -> Result<Vec<String>> {
    if parts.is_empty() {
        anyhow::bail!("Usage: :plugin <command> [args...]");
    }

    let current_exe = std::env::current_exe().unwrap_or_else(|_| "prog".into());
    let mut args = vec![current_exe.display().to_string(), "plugin".to_string()];
    args.extend(parts.iter().map(|part| (*part).to_string()));
    Ok(args)
}

/// Helper to create a registry with config support
fn get_registry(project_root: &Path) -> Result<PluginRegistry> {
    let config_path = project_root.join(".project").join("config.kdl");
    let config = config::load_config(&config_path)?;
    let registry_url = config.plugins.and_then(|p| p.registry_url);
    PluginRegistry::new(project_root, registry_url)
}

fn load_command_manager(
    project_root: &Path,
    command: &str,
) -> Result<crate::plugins::PluginManager> {
    let mut manager = crate::plugins::PluginManager::new(project_root);
    let context = progit_plugin_sdk::traits::PluginContext {
        repo_path: project_root.to_string_lossy().to_string(),
        user: None,
        env: Default::default(),
        config: Default::default(),
    };

    let plugin_dir = project_root.join("plugins");
    manager.load_command_plugins_from_dir(&plugin_dir, &context, command);

    let user_plugins = project_root.join(".progit").join("plugins");
    if user_plugins.exists() {
        manager.load_command_plugins_from_dir(&user_plugins, &context, command);
    }

    Ok(manager)
}

fn print_command_result(command: &str, result: CommandResult) -> Result<()> {
    let CommandResult {
        plugin,
        success,
        output,
        error,
        data: _,
    } = result;

    if let Some(output) = output {
        println!("{output}");
    }

    if !success {
        let error = error.unwrap_or_else(|| format!("plugin command '{command}' failed"));
        anyhow::bail!("{}: {}", plugin, error);
    }

    Ok(())
}

/// Try to run an installed plugin command.
///
/// Returns `Ok(false)` when no installed plugin owns the command. If a plugin
/// owns the command and fails, this returns an error instead of falling back to
/// a built-in implementation.
pub fn try_run_command(project_root: &Path, command: &str, args: &[String]) -> Result<bool> {
    if !plugin_dirs_exist(project_root) {
        return Ok(false);
    }

    let mut manager = load_command_manager(project_root, command)?;
    let Some(result) = manager.dispatch_command(command, args) else {
        return Ok(false);
    };

    print_command_result(command, result)?;
    Ok(true)
}

/// Run an installed plugin command and fail if no plugin handles it.
pub fn run_command(project_root: &Path, command: &str, args: &[String]) -> Result<()> {
    if try_run_command(project_root, command, args)? {
        return Ok(());
    }

    anyhow::bail!(
        "No installed plugin handled command '{}'. Try: prog plugin list",
        command
    )
}

/// List installed plugins
pub fn list(project_root: &Path) -> Result<()> {
    let plugins = collect_installed_plugins(project_root)?;

    if !plugin_dirs_exist(project_root) {
        println!("{} No plugins installed.", "📦".yellow());
        println!("   Install plugins with: prog plugin install <name>");
        return Ok(());
    }

    println!("{} Installed plugins:", "📦".blue());
    println!();

    let mut count = 0;
    for plugin in plugins {
        println!(
            "   {} {} ({}) [{}]",
            "•".green(),
            plugin.name,
            plugin.version.dimmed(),
            plugin.scope.label()
        );
        count += 1;
    }

    if count == 0 {
        println!("   {} No plugins found.", "⚠".yellow());
    } else {
        println!();
        println!("   {} {} plugin(s) installed", "✓".green(), count);
    }

    Ok(())
}

/// Install a plugin
pub fn install(
    project_root: &Path,
    name: &str,
    version: Option<&str>,
    git_url: Option<&str>,
) -> Result<()> {
    let plugin_dir = project_root.join("plugins");
    std::fs::create_dir_all(&plugin_dir)?;

    let source = if let Some(url) = git_url {
        // Direct git URL install
        println!("{} Installing from git: {}", "📥".blue(), url);
        PluginSource::Git {
            url: url.to_string(),
            reference: version.map(|v| v.to_string()),
        }
    } else {
        // Registry install
        println!("{} Searching registry for '{}'...", "🔍".blue(), name);

        let registry = get_registry(project_root)?;

        match registry.find_plugin(name)? {
            Some(manifest) => {
                println!(
                    "{} Found: {} v{}",
                    "✓".green(),
                    manifest.name,
                    manifest.version
                );
                PluginSource::Registry {
                    name: manifest.name.clone(),
                    version: version
                        .map(|v| v.to_string())
                        .unwrap_or(manifest.version.clone()),
                    url: manifest.source_url.clone(),
                    source_path: manifest.source_path.clone(),
                }
            }
            None => {
                anyhow::bail!(
                    "Plugin '{}' not found in registry. Try: prog plugin search {}",
                    name,
                    name
                );
            }
        }
    };

    // Perform the installation
    let installed_path = source.install(&plugin_dir)?;

    // Update lockfile
    let lockfile_path = project_root.join(".project").join("plugins.lock.kdl");
    let mut lockfile = if lockfile_path.exists() {
        Lockfile::load(&lockfile_path)?
    } else {
        Lockfile::new()
    };

    lockfile.add_plugin(name, &source)?;
    lockfile.save(&lockfile_path)?;

    println!("{} Plugin '{}' installed successfully!", "✅".green(), name);
    println!("   Location: {}", installed_path.display());

    Ok(())
}

/// Remove a plugin
pub fn remove(project_root: &Path, name: &str) -> Result<()> {
    let mut removed_count = 0;

    for path in remove_plugin_candidates(project_root, name) {
        if path.extension().map(|ext| ext == "lua").unwrap_or(false) {
            std::fs::remove_file(&path)?;
            println!("{} Removed plugin file: {}", "🗑️".red(), path.display());
        } else {
            if path.is_dir() {
                std::fs::remove_dir_all(&path)?;
            } else {
                std::fs::remove_file(&path)?;
            }
            println!(
                "{} Removed plugin directory: {}",
                "🗑️".red(),
                path.display()
            );
        }
        removed_count += 1;
    }

    if removed_count == 0 {
        anyhow::bail!(
            "Plugin '{}' not found. Use 'prog plugin list' to see installed plugins.",
            name
        );
    }

    // Update lockfile
    let lockfile_path = project_root.join(".project").join("plugins.lock.kdl");
    if lockfile_path.exists() {
        let mut lockfile = Lockfile::load(&lockfile_path)?;
        lockfile.remove_plugin(name);
        lockfile.save(&lockfile_path)?;
    }

    println!("{} Plugin '{}' removed successfully!", "✅".green(), name);

    Ok(())
}

/// Update plugins
pub fn update(project_root: &Path, name: Option<&str>) -> Result<()> {
    let lockfile_path = project_root.join(".project").join("plugins.lock.kdl");

    if !lockfile_path.exists() {
        println!("{} No lockfile found. Nothing to update.", "⚠".yellow());
        return Ok(());
    }

    let lockfile = Lockfile::load(&lockfile_path)?;
    let plugins_to_update: Vec<_> = if let Some(n) = name {
        lockfile
            .plugins()
            .filter(|(pname, _)| *pname == n)
            .collect()
    } else {
        lockfile.plugins().collect()
    };

    if plugins_to_update.is_empty() {
        if let Some(n) = name {
            anyhow::bail!("Plugin '{}' not found in lockfile.", n);
        } else {
            println!("{} No plugins to update.", "⚠".yellow());
            return Ok(());
        }
    }

    println!("{} Checking for updates...", "🔄".blue());

    let registry = get_registry(project_root)?;
    let mut updated = 0;

    for (plugin_name, locked_info) in plugins_to_update {
        if let Some(manifest) = registry.find_plugin(plugin_name)? {
            if manifest.version != locked_info.version {
                println!(
                    "   {} {} -> {}",
                    plugin_name,
                    locked_info.version.dimmed(),
                    manifest.version.green()
                );
                // Re-install with new version
                remove(project_root, plugin_name)?;
                install(project_root, plugin_name, Some(&manifest.version), None)?;
                updated += 1;
            } else {
                println!(
                    "   {} {} (up to date)",
                    plugin_name,
                    locked_info.version.dimmed()
                );
            }
        }
    }

    if updated > 0 {
        println!("{} Updated {} plugin(s).", "✅".green(), updated);
    } else {
        println!("{} All plugins are up to date.", "✅".green());
    }

    Ok(())
}

/// Search the plugin registry
pub fn search(project_root: &Path, query: &str) -> Result<()> {
    println!("{} Searching for '{}'...", "🔍".blue(), query);

    let registry = get_registry(project_root)?;
    let results = registry.search(query)?;

    if results.is_empty() {
        println!("{} No plugins found matching '{}'.", "⚠".yellow(), query);
        return Ok(());
    }

    println!();
    println!("{} Found {} plugin(s):", "📦".blue(), results.len());
    println!();

    for manifest in results {
        println!(
            "   {} {} ({})",
            "•".green(),
            manifest.name.bold(),
            manifest.version
        );
        if !manifest.description.is_empty() {
            println!("     {}", manifest.description.dimmed());
        }
        println!(
            "     Type: {} | Author: {}",
            manifest.plugin_type, manifest.author
        );
        println!();
    }

    Ok(())
}

/// Show plugin info
pub fn info(project_root: &Path, name: &str) -> Result<()> {
    let registry = get_registry(project_root)?;

    match registry.find_plugin(name)? {
        Some(manifest) => {
            println!();
            println!("{} {}", "📦".blue(), manifest.name.bold());
            println!();
            println!("   Version:     {}", manifest.version);
            println!("   Author:      {}", manifest.author);
            println!("   Type:        {}", manifest.plugin_type);
            println!("   License:     {}", manifest.license);
            println!();
            println!("   {}", manifest.description);
            println!();
            println!("   Source: {}", manifest.source_url);
            println!();
            println!("   Install with: prog plugin install {}", manifest.name);
            println!();
        }
        None => {
            anyhow::bail!("Plugin '{}' not found in registry.", name);
        }
    }

    Ok(())
}

/// Update the local registry index
pub fn index_update(project_root: &Path) -> Result<()> {
    println!("{} Updating plugin index...", "🔄".blue());

    let registry = get_registry(project_root)?;
    registry.update_index()?;

    println!("{} Plugin index updated.", "✅".green());

    Ok(())
}

/// Scaffold a new plugin in `<project_root>/plugins/<name>/`.
///
/// Generates `main.lua`, `.progit-plugin.json`, `README.md`, and
/// `.luarc.json` from embedded templates. The `.luarc.json` points at
/// the SDK's LuaCATS stubs so the author gets editor autocomplete from
/// the first edit.
pub fn new_plugin(project_root: &Path, name: &str, author: Option<&str>) -> Result<()> {
    if !is_kebab_case(name) {
        anyhow::bail!(
            "plugin name '{}' is invalid: use lowercase letters, digits, and hyphens (e.g. 'jira-sync')",
            name
        );
    }

    let target_dir = project_root.join("plugins").join(name);
    if target_dir.exists() {
        anyhow::bail!("plugins/{} already exists", name);
    }

    let resolved_author = author
        .map(|s| s.to_string())
        .or_else(detect_git_user_name)
        .unwrap_or_else(|| "Anonymous".to_string());

    let description = format!("{} plugin", name);
    let sdk_version = progit_plugin_sdk::SDK_API_VERSION;

    let render = |tmpl: &str| -> String {
        tmpl.replace("{{name}}", name)
            .replace("{{author}}", &resolved_author)
            .replace("{{description}}", &description)
            .replace("{{sdk_version}}", sdk_version)
    };

    std::fs::create_dir_all(&target_dir)
        .with_context(|| format!("Failed to create {}", target_dir.display()))?;

    let main_lua = render(include_str!("templates/main.lua.tmpl"));
    let manifest = render(include_str!("templates/manifest.json.tmpl"));
    let readme = render(include_str!("templates/README.md.tmpl"));
    let luarc = render(include_str!("templates/luarc.json.tmpl"));

    std::fs::write(target_dir.join("main.lua"), main_lua)?;
    std::fs::write(target_dir.join(".progit-plugin.json"), manifest)?;
    std::fs::write(target_dir.join("README.md"), readme)?;
    std::fs::write(target_dir.join(".luarc.json"), luarc)?;

    println!(
        "{} Scaffolded plugin {} at {}",
        "✓".green(),
        name.bold(),
        target_dir.display()
    );
    println!();
    println!("  Next steps:");
    println!("    1. Edit plugins/{}/main.lua", name);
    println!(
        "    2. Edit plugins/{}/.progit-plugin.json (capabilities, hooks)",
        name
    );
    println!("    3. Run: prog plugin list   # confirm it loads");
    println!();
    Ok(())
}

fn is_kebab_case(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .next()
            .map(|c| c.is_ascii_lowercase())
            .unwrap_or(false)
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn detect_git_user_name() -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["config", "--get", "user.name"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn tui_plugin_args_wrap_current_executable() {
        let args = tui_command_args(&["sober", "preflight", "--base", "HEAD"]).unwrap();

        assert!(args.len() >= 6);
        assert_eq!(args[1], "plugin");
        assert_eq!(args[2..], ["sober", "preflight", "--base", "HEAD"]);
    }

    #[test]
    fn tui_plugin_args_require_command() {
        let err = tui_command_args(&[]).unwrap_err().to_string();

        assert!(err.contains("Usage: :plugin"));
    }

    #[test]
    fn discover_plugins_looks_in_project_and_user_plugin_dirs() {
        let temp = tempfile::tempdir().unwrap();
        let project_root = temp.path();

        let project_dir = project_root.join("plugins");
        let user_dir = project_root.join(".progit").join("plugins");
        fs::create_dir_all(&project_dir).unwrap();
        fs::create_dir_all(&user_dir).unwrap();

        fs::write(project_dir.join("project.lua"), "return {}").unwrap();
        fs::create_dir_all(user_dir.join("user-plugin")).unwrap();
        fs::write(
            user_dir.join("user-plugin").join(".progit-plugin.json"),
            "{}",
        )
        .unwrap();

        let discovered = collect_installed_plugins(project_root).unwrap();

        assert_eq!(discovered.len(), 2);
        let project = discovered
            .iter()
            .find(|plugin| plugin.name == "project")
            .unwrap_or_else(|| panic!("missing project plugin"));
        assert_eq!(project.scope, PluginInstallScope::Project);
        assert_eq!(project.version, "local");

        let user = discovered
            .iter()
            .find(|plugin| plugin.name == "user-plugin")
            .unwrap_or_else(|| panic!("missing user plugin"));
        assert_eq!(user.scope, PluginInstallScope::User);
    }

    #[test]
    fn remove_deletes_plugin_from_all_install_locations() {
        let temp = tempfile::tempdir().unwrap();
        let project_root = temp.path();

        let project_dir = project_root.join("plugins");
        let user_dir = project_root.join(".progit").join("plugins");
        fs::create_dir_all(&project_dir).unwrap();
        fs::create_dir_all(&user_dir).unwrap();

        fs::write(project_dir.join("shared.lua"), "return {}").unwrap();
        fs::create_dir_all(user_dir.join("shared")).unwrap();
        fs::write(user_dir.join("shared").join(".progit-plugin.json"), "{}").unwrap();

        remove(project_root, "shared").unwrap();

        assert!(!project_dir.join("shared.lua").exists());
        assert!(!user_dir.join("shared").exists());
    }
}
