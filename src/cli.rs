//! CLI command handlers for plugin and hooks subcommands

use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};

use colored::*;

use crate::hooks;

// Re-use the CLI enum types defined in main.rs (imported via super:: since main.rs is the crate root)
use super::{HooksAction, IndexAction, PluginAction};

/// Try to dispatch a hooks command to the plugin system first.
/// Returns true if a plugin handled the command, false otherwise.
fn try_plugin_hooks_command(command: &str, args: &[String]) -> bool {
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let plugin_dir = project_root.join("plugins");

    if !plugin_dir.exists() {
        return false;
    }

    let mut manager = crate::plugins::PluginManager::new(&project_root);
    
    let context = progit_plugin_sdk::traits::PluginContext {
        repo_path: project_root.to_string_lossy().to_string(),
        user: None,
        env: Default::default(),
        config: Default::default(),
    };
    
    if manager.load_all(&context).is_err() {
        return false;
    }

    match manager.dispatch_command(command, args) {
        Some(result) => {
            if let Some(output) = result.output {
                println!("{}", output);
            }
            if !result.success {
                if let Some(error) = result.error {
                    eprintln!("{} {}", "Error:".red(), error);
                }
                std::process::exit(1);
            }
            true
        }
        None => false,
    }
}

/// Handle plugin CLI commands
pub(crate) fn handle_plugin_command(action: PluginAction) -> Result<()> {
    use progit_plugin_sdk::prelude::{LuaPlugin, Plugin};

    // Plugin directory for legacy file-based fallback installs
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
            use crate::plugins::cli as plugin_cli;
            let project_root = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            plugin_cli::remove(&project_root, &name)?;
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

        PluginAction::New { name, author } => {
            use crate::plugins::cli as plugin_cli;
            let project_root = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            plugin_cli::new_plugin(&project_root, &name, author.as_deref())?;
        }
        PluginAction::Verify { name } => {
            crate::marketplace::cli::handle_plugin_verify(&name)?;
        }
    }

    Ok(())
}

/// Handle hooks CLI commands
pub(crate) fn handle_hooks_command(action: HooksAction, repo_root: &Path) -> Result<()> {
    // Try plugin system first for subcommands that plugins can handle
    match &action {
        HooksAction::Validate { hook_type, value } => {
            // Plugin expects: ["validate", <hook_type>, <value>]
            let mut args: Vec<String> = vec!["validate".to_string(), hook_type.clone()];
            if let Some(v) = value {
                args.push(v.clone());
            }
            if try_plugin_hooks_command("hooks", &args) {
                return Ok(());
            }
        }
        HooksAction::Status => {
            // Plugin expects: ["status"]
            let args = vec!["status".to_string()];
            if try_plugin_hooks_command("hooks", &args) {
                return Ok(());
            }
        }
        HooksAction::Install => {
            // Plugin expects: ["install"]
            let args = vec!["install".to_string()];
            if try_plugin_hooks_command("hooks", &args) {
                return Ok(());
            }
        }
        HooksAction::Uninstall => {
            // Plugin expects: ["uninstall"]
            let args = vec!["uninstall".to_string()];
            if try_plugin_hooks_command("hooks", &args) {
                return Ok(());
            }
        }
    }

    // Fall back to built-in hooks
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
        HooksAction::Validate { hook_type, value } => {
            if hook_type == "commit-msg" || hook_type == "commit" {
                if let Some(msg) = value {
                    let refs = hooks::parse_issue_references(&msg);
                    println!();
                    println!("{} Validating commit message...", "🔍".blue());
                    println!();
                    if refs.is_empty() {
                        println!("  {} No issue references found", "ℹ️".dimmed());
                    } else {
                        println!("{} Found {} issue reference(s):", "📌".cyan(), refs.len());
                        for r in &refs {
                            let action_str = match r.action {
                                hooks::IssueAction::Close => "Close".to_string(),
                                hooks::IssueAction::Reference => "Reference".to_string(),
                                hooks::IssueAction::Mention => "Mention".to_string(),
                            };
                            println!("  {} #{} ({})", action_str.green(), r.issue_id, action_str);
                        }
                    }
                } else {
                    println!("{} Usage: prog hooks validate commit-msg \"closes #123\"", "ℹ️".yellow());
                }
            } else if hook_type == "branch" {
                println!("{} Branch validation - use git-hooks plugin for full validation", "ℹ️".cyan());
            } else {
                println!("{} Unknown hook type: {}", "⚠️".yellow(), hook_type);
                println!("Valid types: commit-msg, branch");
            }
        }
    }

    Ok(())
}
