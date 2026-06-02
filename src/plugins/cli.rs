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
use std::path::Path;

use super::lockfile::Lockfile;
use super::manager::CommandResult;
use super::registry::{PluginRegistry, PluginSource};
use crate::storage::config;

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
        let error = error
            .unwrap_or_else(|| format!("plugin command '{command}' failed"));
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
    let plugin_dir = project_root.join("plugins");
    let user_plugin_dir = project_root.join(".progit").join("plugins");
    if !plugin_dir.exists() && !user_plugin_dir.exists() {
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
    let plugin_dir = project_root.join("plugins");

    if !plugin_dir.exists() {
        println!("{} No plugins installed.", "📦".yellow());
        println!("   Install plugins with: prog plugin install <name>");
        return Ok(());
    }

    let mut count = 0;

    // Check for lockfile
    let lockfile_path = project_root.join(".project").join("plugins.lock.kdl");
    let lockfile = if lockfile_path.exists() {
        Lockfile::load(&lockfile_path).ok()
    } else {
        None
    };

    println!("{} Installed plugins:", "📦".blue());
    println!();

    for entry in std::fs::read_dir(&plugin_dir)? {
        let entry = entry?;
        let path = entry.path();

        // Check for .lua files or plugin directories
        if path.extension().map(|e| e == "lua").unwrap_or(false) {
            let name = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");

            // Get version from lockfile if available
            let version = lockfile.as_ref()
                .and_then(|lf| lf.get_version(name))
                .unwrap_or_else(|| "local".to_string());

            println!("   {} {} ({})", "•".green(), name, version.dimmed());
            count += 1;
        } else if path.is_dir() {
            // Check for plugin manifest in directory
            let manifest_path = path.join(".progit-plugin.json");
            if manifest_path.exists() {
                let name = path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");

                let version = lockfile.as_ref()
                    .and_then(|lf| lf.get_version(name))
                    .unwrap_or_else(|| "local".to_string());

                println!("   {} {} ({})", "•".green(), name, version.dimmed());
                count += 1;
            }
        }
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
pub fn install(project_root: &Path, name: &str, version: Option<&str>, git_url: Option<&str>) -> Result<()> {
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
                println!("{} Found: {} v{}", "✓".green(), manifest.name, manifest.version);
                PluginSource::Registry {
                    name: manifest.name.clone(),
                    version: version.map(|v| v.to_string()).unwrap_or(manifest.version.clone()),
                    url: manifest.source_url.clone(),
                    source_path: manifest.source_path.clone(),
                }
            }
            None => {
                anyhow::bail!("Plugin '{}' not found in registry. Try: prog plugin search {}", name, name);
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
    let plugin_dir = project_root.join("plugins");

    // Check for .lua file
    let lua_path = plugin_dir.join(format!("{}.lua", name));
    let dir_path = plugin_dir.join(name);

    let removed = if lua_path.exists() {
        std::fs::remove_file(&lua_path)?;
        println!("{} Removed plugin file: {}", "🗑️".red(), lua_path.display());
        true
    } else if dir_path.exists() && dir_path.is_dir() {
        std::fs::remove_dir_all(&dir_path)?;
        println!("{} Removed plugin directory: {}", "🗑️".red(), dir_path.display());
        true
    } else {
        false
    };

    if !removed {
        anyhow::bail!("Plugin '{}' not found. Use 'prog plugin list' to see installed plugins.", name);
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
        lockfile.plugins()
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
                println!("   {} {} -> {}", plugin_name, locked_info.version.dimmed(), manifest.version.green());
                // Re-install with new version
                remove(project_root, plugin_name)?;
                install(project_root, plugin_name, Some(&manifest.version), None)?;
                updated += 1;
            } else {
                println!("   {} {} (up to date)", plugin_name, locked_info.version.dimmed());
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
        println!("   {} {} ({})", "•".green(), manifest.name.bold(), manifest.version);
        if !manifest.description.is_empty() {
            println!("     {}", manifest.description.dimmed());
        }
        println!("     Type: {} | Author: {}", manifest.plugin_type, manifest.author);
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
    println!("    2. Edit plugins/{}/.progit-plugin.json (capabilities, hooks)", name);
    println!("    3. Run: prog plugin list   # confirm it loads");
    println!();
    Ok(())
}

fn is_kebab_case(s: &str) -> bool {
    !s.is_empty()
        && s.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false)
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
