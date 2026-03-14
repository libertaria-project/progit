//! CLI command handlers for plugin and hooks subcommands

use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};

use colored::*;

use crate::hooks;

// Re-use the CLI enum types defined in main.rs (imported via super:: since main.rs is the crate root)
use super::{HooksAction, IndexAction, PluginAction};

/// Handle plugin CLI commands
pub(crate) fn handle_plugin_command(action: PluginAction) -> Result<()> {
    use progit_plugin_sdk::prelude::{LuaPlugin, Plugin};

    // Plugin directories
    let home_plugins = dirs::home_dir()
        .map(|h| h.join(".progit").join("plugins"))
        .unwrap_or_else(|| PathBuf::from(".progit/plugins"));
    let local_plugins = PathBuf::from(".progit/plugins");

    match action {
        PluginAction::List => {
            use crate::plugins::cli as plugin_cli;
            let project_root = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            plugin_cli::list(&project_root)?;
        }

        PluginAction::Install { name, version, git } => {
            use crate::plugins::cli as plugin_cli;

            let project_root = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."));

            // Use the registry system from plugins::cli
            if git {
                // Git URL installation
                plugin_cli::install(
                    &project_root,
                    &name,
                    version.as_deref(),
                    Some(&name),
                )?;
            } else {
                // Try registry first
                match plugin_cli::install(
                    &project_root,
                    &name,
                    version.as_deref(),
                    None,
                ) {
                    Ok(()) => {},
                    Err(_) => {
                        // Fall back to local file installation
                        let source_path = PathBuf::from(&name);
                        if !source_path.exists() {
                            return Err(anyhow!(
                                "Plugin '{}' not found in registry and not found as file",
                                name
                            ));
                        }

                        // Validate it's a valid Lua plugin
                        let plugin = LuaPlugin::load(&source_path)
                            .with_context(|| format!("Failed to load plugin from {}", name))?;
                        let meta = plugin.metadata();

                        // Create plugins directory if needed
                        std::fs::create_dir_all(&local_plugins)?;

                        // Copy to plugins directory
                        let dest = local_plugins.join(format!("{}.lua", meta.name));
                        std::fs::copy(&source_path, &dest)
                            .with_context(|| "Failed to copy plugin")?;

                        println!("{} Installed {} v{}", "✓".green(), meta.name.green(), meta.version);
                        println!("  Location: {}", dest.display());
                    }
                }
            }
        }

        PluginAction::Remove { name } => {
            let mut removed = false;

            for dir in [&local_plugins, &home_plugins] {
                let plugin_path = dir.join(format!("{}.lua", name));
                if plugin_path.exists() {
                    std::fs::remove_file(&plugin_path)?;
                    println!("{} Removed plugin: {}", "✓".green(), name);
                    removed = true;
                    break;
                }
            }

            if !removed {
                return Err(anyhow!("Plugin '{}' not found", name));
            }
        }

        PluginAction::Update { name } => {
            use crate::plugins::cli as plugin_cli;
            let project_root = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            plugin_cli::update(&project_root, name.as_deref())?;
        }

        PluginAction::Search { query } => {
            use crate::plugins::cli as plugin_cli;
            let project_root = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            plugin_cli::search(&project_root, &query)?;
        }

        PluginAction::Info { name } => {
            use crate::plugins::cli as plugin_cli;
            let project_root = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            plugin_cli::info(&project_root, &name)?;
        }

        PluginAction::Index { action: index_action } => {
            use crate::plugins::cli as plugin_cli;
            let project_root = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            match index_action {
                IndexAction::Update => {
                    plugin_cli::index_update(&project_root)?;
                }
            }
        }
    }

    Ok(())
}

/// Handle hooks CLI commands
pub(crate) fn handle_hooks_command(action: HooksAction, repo_root: &Path) -> Result<()> {
    match action {
        HooksAction::Install => {
            println!("{} Installing ProGit hooks...", "🔧".blue());
            match hooks::install_hooks(repo_root) {
                Ok(installed) => {
                    for hook in &installed {
                        println!("  {} Installed {}", "✓".green(), hook.filename());
                    }
                    println!();
                    println!("{} Hooks will auto-update issues based on commit messages:", "ℹ️".cyan());
                    println!("  {} closes #123, fixes #123, resolves #123 → marks Done", "•".dimmed());
                    println!("  {} refs #123, see #123, re #123 → marks In Progress", "•".dimmed());
                    println!("  {} #123 (bare reference) → no status change", "•".dimmed());
                }
                Err(e) => {
                    return Err(anyhow!("Failed to install hooks: {}", e));
                }
            }
        }
        HooksAction::Uninstall => {
            println!("{} Uninstalling ProGit hooks...", "🔧".blue());
            match hooks::uninstall_hooks(repo_root) {
                Ok(uninstalled) => {
                    if uninstalled.is_empty() {
                        println!("  {} No ProGit hooks were installed", "ℹ️".cyan());
                    } else {
                        for hook in &uninstalled {
                            println!("  {} Removed {}", "✓".green(), hook.filename());
                        }
                    }
                }
                Err(e) => {
                    return Err(anyhow!("Failed to uninstall hooks: {}", e));
                }
            }
        }
        HooksAction::Status => {
            println!("{}", "Git Hooks Status".bold());
            println!("{}", "─".repeat(40));
            match hooks::hooks_status(repo_root) {
                Ok(status) => {
                    for (hook, installed) in status {
                        let status_str = if installed {
                            "installed".green()
                        } else {
                            "not installed".dimmed()
                        };
                        println!("  {:<15} {}", hook.filename(), status_str);
                    }
                }
                Err(e) => {
                    return Err(anyhow!("Failed to check hook status: {}", e));
                }
            }
        }
    }

    Ok(())
}
