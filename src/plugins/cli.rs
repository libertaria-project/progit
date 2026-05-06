// SPDX-License-Identifier: EUPL-1.2
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

use anyhow::Result;
use colored::*;
use std::path::Path;

use super::registry::{PluginRegistry, PluginSource};
use super::lockfile::Lockfile;
use crate::storage::config;

/// Helper to create a registry with config support
fn get_registry(project_root: &Path) -> Result<PluginRegistry> {
    let config_path = project_root.join(".project").join("config.kdl");
    let config = config::load_config(&config_path)?;
    let registry_url = config.plugins.and_then(|p| p.registry_url);
    PluginRegistry::new(project_root, registry_url)
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
