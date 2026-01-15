//! ProjectsTUI - Lean Git Issue Tracker
//!
//! Terminal cockpit f#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

mod command;
mod diff;
mod fuzzy;
mod git;
mod hooks;
mod issue;
mod mr;
mod panopticum;
mod plugins;
mod rebase;
mod storage;
mod sync;
mod tui;
mod virtual_branch;
mod agent;

use anyhow::Result;
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, MouseEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::{Path, PathBuf};

use crate::git::detect_repo;
use crate::issue::{Issue, Status};
use crate::storage::{delete_issue, load_issues, paths, save_issue, sync_kdl_to_json};
use crate::sync::SyncProvider;
use crate::tui::{handle_key, handle_mouse, render, App, KeyAction, UIAreas};
use anyhow::{anyhow, Context};

use colored::*;

/// ProGit - Lean Git Issue Tracker
#[derive(Parser)]
#[command(name = "prog")]
#[command(version)]
#[command(about = "Terminal cockpit for developers, sync bridge to the cloud", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Bidirectional sync with forge (auto-detect changes)
    Sync {
        #[command(subcommand)]
        action: Option<SyncAction>,
    },
    /// Emergency cleanup of duplicate issues
    Clean,
    /// Toggle blocked status for an issue
    Block {
        /// Issue ID (short or full UUID)
        id: String,
    },
    /// Set due date for an issue (YYYY-MM-DD or "clear")
    Due {
        /// Issue ID (short or full UUID)
        id: String,
        /// Due date (YYYY-MM-DD format) or "clear" to remove
        date: String,
    },
    /// Manage git branches
    Branch {
        #[command(subcommand)]
        action: Option<BranchAction>,
    },
    /// Manage merge requests
    Mr {
        #[command(subcommand)]
        action: Option<MrAction>,
    },
    /// Manage plugins
    Plugin {
        #[command(subcommand)]
        action: PluginAction,
    },
    /// Internal: Interactive rebase editor (triggered by git)
    #[command(hide = true)]
    RebaseEditor {
        /// Path to the git-rebase-todo file
        path: String,
    },
    /// Manage git hooks integration
    Hooks {
        #[command(subcommand)]
        action: HooksAction,
    },
}

#[derive(Subcommand)]
enum PluginAction {
    /// List installed plugins
    List,
    /// Install a plugin from registry or git URL
    Install {
        /// Plugin name or git URL (with --git flag)
        name: String,
        /// Specific version to install
        #[arg(short, long)]
        version: Option<String>,
        /// Install from git URL directly
        #[arg(long)]
        git: bool,
    },
    /// Remove an installed plugin
    Remove {
        /// Plugin name to remove
        name: String,
    },
    /// Update installed plugins
    Update {
        /// Specific plugin to update (updates all if not specified)
        name: Option<String>,
    },
    /// Search the plugin registry
    Search {
        /// Search query
        query: String,
    },
    /// Show plugin information
    Info {
        /// Plugin name
        name: String,
    },
    /// Update the local plugin index
    Index {
        #[command(subcommand)]
        action: IndexAction,
    },
}

#[derive(Subcommand)]
enum IndexAction {
    /// Update the local plugin index from remote
    Update,
}

#[derive(Subcommand)]
enum HooksAction {
    /// Install ProGit git hooks
    Install,
    /// Uninstall ProGit git hooks
    Uninstall,
    /// Show status of installed hooks
    Status,
}

#[derive(Subcommand)]
enum BranchAction {
    /// List local branches
    List,
    /// Switch to a branch
    Switch { name: String },
    /// Create a new branch
    Create { name: String },
    /// Delete a branch
    Delete { name: String },
    /// Manage remote branches
    Remote {
        #[command(subcommand)]
        action: RemoteBranchAction,
    },
}

#[derive(Subcommand)]
enum RemoteBranchAction {
    /// List remote branches
    List,
    /// Create/Push a remote branch
    Create {
        /// Branch name
        name: String,
    },
}

#[derive(Subcommand)]
enum MrAction {
    /// List open merge requests (requires sync config)
    List,
    /// Create a new merge request (interactive or from args)
    Create {
        /// Target branch (default: main/master)
        #[arg(short = 'b', long)]
        target: Option<String>,
        /// Title (default: from last commit or branch name)
        #[arg(short, long)]
        title: Option<String>,
    },
    /// Approve a merge request (LGTM review)
    Approve {
        /// MR number to approve
        id: u64,
    },
    /// Accept and merge a merge request
    Merge {
        /// MR number to merge
        id: u64,
    },
    /// Reject a merge request (close without merging)
    Reject {
        /// MR number to reject
        id: u64,
    },
}

#[derive(Subcommand)]
enum SyncAction {
    /// Push local issues to remote forge
    Push,
    /// Pull remote forge issues to local
    Pull,
}

fn main() -> Result<()> {
    // Initialize Logger
    tui_logger::init_logger(log::LevelFilter::Trace).unwrap();
    tui_logger::set_default_level(log::LevelFilter::Info);

    // 1. Detect Root
    let project_root = find_project_root()?;

    // Check if we should initialize
    let project_dir = project_root.join(storage::paths::PROJECT_DIR);
    if !project_dir.exists() {
        // If no .project exists, we require either a git repository OR a PANOPTICUM.kdl
        let has_git = crate::git::detect_repo(&project_root)?.is_some();
        let has_panopticum = project_root.join("PANOPTICUM.kdl").exists();

        if !has_git && !has_panopticum {
            println!("{} No git repository or PANOPTICUM.kdl found.", "❌".red());
            println!("   ProGit requires either:");
            println!("   - A git repository (run 'git init'), or");
            println!("   - A PANOPTICUM.kdl file (infrastructure repo)");
            return Ok(());
        }
    }

    // 2. Migration Check (.projects -> Split Core)
    storage::check_and_migrate(&project_root)?;

    // 3. Auto-Initialization (Ensure .project and .progit exist)
    initialize_workspace(&project_root)?;

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Sync { action }) => {
            // Load config
            let project_root = find_project_root()?;
            let config_path = project_root.join(paths::config_file());
            let config = storage::config::load_config(&config_path)?;

            let mut sync_config = config.sync.context(
                "No sync configuration found. Please add a 'sync' block to .projects/config.kdl",
            )?;

            // Auto-configure based on remote
            let cwd = std::env::current_dir()?;
            auto_configure(&mut sync_config, &cwd);

            // Choose provider
            let provider = sync::create_provider(sync_config.clone());

            // Sync command handling - use modern StorageEngine (issues.json)
            let mut engine = storage::engine::StorageEngine::new(&project_root);
            engine.load()?;

            match action {
                Some(SyncAction::Push) => {
                    let mut issues = engine.issues().to_vec();

                    println!(
                        "{} Authenticating with {}...",
                        "🔄".blue(),
                        sync_config.provider
                    );
                    provider.login()?;

                    println!(
                        "{} Pushing {} issues to remote...",
                        "🔄".blue(),
                        issues.len()
                    );
                    provider.push(&mut issues)?;

                    // Save issues back to persist new remote IDs
                    *engine.issues_mut() = issues.clone();
                    engine.save()?;
                    println!(
                        "{} Saved {} issues with remote links.",
                        "💾".green(),
                        issues.len()
                    );

                    // Delete remote issues not in local
                    let deleted = provider.delete_missing(&issues)?;
                    if deleted > 0 {
                        println!("{} Deleted {} orphaned remote issues.", "🗑️".red(), deleted);
                    }

                    println!("{} Push complete.", "✅".green());
                }
                Some(SyncAction::Pull) => {
                    let local_issues = engine.issues().to_vec();

                    println!(
                        "{} Authenticating with {}...",
                        "🔄".blue(),
                        sync_config.provider
                    );
                    provider.login()?;

                    println!("{} Pulling issues from remote...", "🔄".blue());
                    let remote_issues = provider.pull()?;

                    println!(
                        "{} Pulled {} issues. Merging...",
                        "📥".blue(),
                        remote_issues.len()
                    );

                    let provider_name = &sync_config.provider;
                    let merged_issues =
                        sync::merge_issues(&local_issues, remote_issues, provider_name);

                    // Save merged set
                    *engine.issues_mut() = merged_issues.clone();
                    engine.save()?;
                    println!(
                        "{} Saved {} merged issues.",
                        "💾".green(),
                        merged_issues.len()
                    );
                    println!("{} Pull complete.", "✅".green());
                }
                None => {
                    // Default: bidirectional sync (push then pull)
                    let mut issues = engine.issues().to_vec();

                    println!(
                        "{} Authenticating with {}...",
                        "🔄".blue(),
                        sync_config.provider
                    );
                    provider.login()?;

                    // Push first
                    println!("{} Pushing {} local issues...", "⬆️".blue(), issues.len());
                    provider.push(&mut issues)?;

                    // Update engine with any new remote IDs
                    *engine.issues_mut() = issues.clone();

                    // Then pull
                    println!("{} Pulling remote updates...", "⬇️".blue());
                    let remote_issues = provider.pull()?;
                    let merged =
                        sync::merge_issues(engine.issues(), remote_issues, &sync_config.provider);

                    *engine.issues_mut() = merged;
                    engine.save()?;

                    println!(
                        "{} Sync complete: {} issues.",
                        "✅".green(),
                        engine.issues().len()
                    );
                }
            }
        }
        Some(Commands::Branch { action }) => {
            let cwd = std::env::current_dir()?;
            let repo_info = detect_repo(&cwd)?.context("Not a git repository")?;

            match action {
                Some(BranchAction::List) | None => {
                    println!("{} Branches in {}:", "🌿".green(), repo_info.path.blue());
                    for branch in repo_info.branches {
                        if branch == repo_info.branch {
                            println!("  {} {}", "*".green(), branch.green().bold());
                        } else {
                            println!("    {}", branch);
                        }
                    }
                }
                Some(BranchAction::Switch { name }) => {
                    crate::git::switch_branch(&cwd, &name)?;
                    println!(
                        "{} Switched to branch {}",
                        "✅".green(),
                        name.green().bold()
                    );
                }
                Some(BranchAction::Create { name }) => {
                    crate::git::create_branch(&project_root, &name)?;
                    println!(
                        "{} Created and switched to branch {}",
                        "✅".green(),
                        name.cyan()
                    );
                }
                Some(BranchAction::Delete { name }) => {
                    crate::git::delete_branch(&project_root, &name)?;
                    println!("{} Deleted branch {}", "🗑️".green(), name.cyan());
                }
                Some(BranchAction::Remote { action }) => {
                    match action {
                        RemoteBranchAction::List => {
                            // Get remote URL for context
                            let remote_url = crate::git::get_remote_url(&project_root, "origin")
                                .ok()
                                .flatten()
                                .unwrap_or_else(|| "unknown".to_string());

                            // Parse project name from URL
                            let project_name = if let Some((_, owner, repo)) =
                                crate::git::parse_git_url(&remote_url)
                            {
                                format!("{}/{}", owner, repo)
                            } else {
                                "unknown/unknown".to_string()
                            };

                            let branches = crate::git::list_remote_branches(&project_root)?;
                            println!(
                                "{} Remote Branches: {} ({})",
                                "🌐".blue(),
                                project_name.cyan(),
                                remote_url.dimmed()
                            );
                            for branch in branches {
                                println!("  - {}", branch);
                            }
                        }
                        RemoteBranchAction::Create { name } => {
                            // Default to create on "origin" for now
                            match crate::git::create_remote_branch(&project_root, &name, None) {
                                Ok(_) => println!(
                                    "{} Pushed HEAD to new remote branch {}",
                                    "🚀".green(),
                                    name.cyan()
                                ),
                                Err(e) => {
                                    println!("{} Failed to create remote branch: {}", "❌".red(), e)
                                }
                            }
                        }
                    }
                }
            }
        }
        Some(Commands::Mr { action }) => {
            // Load config for provider
            let project_root = find_project_root()?;
            let config_path = project_root.join(paths::config_file());
            let config = storage::config::load_config(&config_path)?;
            let cwd = std::env::current_dir()?;
            let repo_info = detect_repo(&cwd)?.context("Not a git repository")?;

            let mut sync_config = config.sync.context(
                "No sync configuration found. MR commands require a configured provider.",
            )?;

            // Auto-configure based on remote
            auto_configure(&mut sync_config, &cwd);

            let provider = sync::create_provider(sync_config.clone());
            provider.login()?;

            match action {
                Some(MrAction::List) | None => {
                    println!("{} Fetching open Merge Requests...", "🔄".blue());
                    let mrs = provider.list_mrs()?;
                    if mrs.is_empty() {
                        println!("No open MRs found.");
                    } else {
                        println!("{} Open Merge Requests:", "🔀".green());
                        for mr in mrs {
                            let id_display = if let Some(rid) = mr.remote_id {
                                format!("!{}", rid)
                            } else {
                                format!("!{}", &mr.id[..8])
                            };

                            println!(
                                "  {:<6} {} {}",
                                id_display.cyan(),
                                mr.title.bold(),
                                format!("({} -> {})", mr.source_branch, mr.target_branch).dimmed()
                            );
                        }
                    }
                }
                Some(MrAction::Create { target, title }) => {
                    let source_branch = repo_info.branch.clone();
                    let target_branch = target.unwrap_or_else(|| "main".to_string());

                    let mr_title = if let Some(t) = title {
                        t
                    } else {
                        // TODO: Use last commit message if possible, or branch name
                        source_branch.clone()
                    };

                    println!(
                        "{} Creating MR: {} -> {}",
                        "🔄".blue(),
                        source_branch.cyan(),
                        target_branch.cyan()
                    );

                    let mr =
                        crate::mr::MergeRequest::new(&source_branch, &target_branch, &mr_title);
                    match provider.create_mr(&mr) {
                        Ok(id) => println!("{} Created MR !{}", "✅".green(), id),
                        Err(e) => println!("{} Failed to create MR: {}", "❌".red(), e),
                    }
                }
                Some(MrAction::Approve { id }) => {
                    println!("{} Approving MR !{}...", "🔄".blue(), id);
                    match provider.approve_mr(id) {
                        Ok(_) => println!("{} Approved MR !{} (LGTM)", "👍".green(), id),
                        Err(e) => println!("{} Failed to approve: {}", "❌".red(), e),
                    }
                }
                Some(MrAction::Merge { id }) => {
                    println!("{} Merging MR !{}...", "🔄".blue(), id);
                    match provider.merge_mr(id) {
                        Ok(_) => println!("{} Accepted & Merged MR !{}", "✅".green(), id),
                        Err(e) => println!("{} Failed to merge: {}", "❌".red(), e),
                    }
                }
                Some(MrAction::Reject { id }) => {
                    println!(
                        "{} Rejecting MR !{} (closing without merge)...",
                        "🔄".blue(),
                        id
                    );
                    match provider.close_mr(id) {
                        Ok(_) => println!("{} Rejected MR !{}", "❌".yellow(), id),
                        Err(e) => println!("{} Failed to reject: {}", "❌".red(), e),
                    }
                }
            }
        }
        Some(Commands::Clean) => {
            println!("{} Starting emergency cleanup...", "🧹".yellow());
            storage::cleanup_duplicates(&find_project_root()?)?;
            println!("{} Cleanup complete.", "✅".green());
        }
        Some(Commands::Block { id }) => {
            let project_root = find_project_root()?;
            let mut engine = storage::engine::StorageEngine::new(&project_root);
            engine.load()?;

            // Find issue by full or short ID
            if let Some(issue) = engine
                .issues_mut()
                .iter_mut()
                .find(|i| i.id == id || i.id.starts_with(&id))
            {
                issue.blocked = !issue.blocked;
                issue.updated = chrono::Utc::now();
                let blocked_str = if issue.blocked {
                    "BLOCKED".red().bold()
                } else {
                    "UNBLOCKED".green().bold()
                };
                println!(
                    "{} Issue {} marked as {}",
                    "🔥".yellow(),
                    issue.short_id().bold(),
                    blocked_str
                );
                engine.save()?;
            } else {
                return Err(anyhow!("Issue '{}' not found", id));
            }
        }
        Some(Commands::Due { id, date }) => {
            let project_root = find_project_root()?;
            let mut engine = storage::engine::StorageEngine::new(&project_root);
            engine.load()?;

            // Find issue by full or short ID
            if let Some(issue) = engine
                .issues_mut()
                .iter_mut()
                .find(|i| i.id == id || i.id.starts_with(&id))
            {
                if date.to_lowercase() == "clear" {
                    issue.due = None;
                    println!(
                        "{} Due date cleared for issue {}",
                        "⏰".green(),
                        issue.short_id().bold()
                    );
                } else {
                    // Parse date (YYYY-MM-DD format)
                    let parsed_date = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")
                        .context("Invalid date format. Use YYYY-MM-DD")?;
                    let due_datetime = parsed_date
                        .and_hms_opt(23, 59, 59)
                        .context("Invalid time")?;
                    issue.due = Some(chrono::DateTime::from_naive_utc_and_offset(
                        due_datetime,
                        chrono::Utc,
                    ));
                    println!(
                        "{} Due date set to {} for issue {}",
                        "⏰".green(),
                        date.cyan(),
                        issue.short_id().bold()
                    );
                }
                issue.updated = chrono::Utc::now();
                engine.save()?;
            } else {
                return Err(anyhow!("Issue '{}' not found", id));
            }
        }
        Some(Commands::Plugin { action }) => {
            handle_plugin_command(action)?;
        }
        Some(Commands::Hooks { action }) => {
            handle_hooks_command(action, &project_root)?;
        }
        Some(Commands::RebaseEditor { path }) => {
            crate::rebase::run(&path)?;
        }
        None => {
            // No command - run TUI
            start_tui()?;
        }
    }

    Ok(())
}

/// Handle plugin CLI commands
fn handle_plugin_command(action: PluginAction) -> Result<()> {
    use progit_plugin_sdk::prelude::{LuaPlugin, Plugin};

    // Plugin directories
    let home_plugins = dirs::home_dir()
        .map(|h| h.join(".progit").join("plugins"))
        .unwrap_or_else(|| PathBuf::from(".progit/plugins"));
    let local_plugins = PathBuf::from(".progit/plugins");

    match action {
        PluginAction::List => {
            println!("{}", "Installed Plugins".bold());
            println!("{}", "─".repeat(50));

            let mut found = false;
            for dir in [&local_plugins, &home_plugins] {
                if dir.exists() {
                    if let Ok(entries) = std::fs::read_dir(dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.extension().is_some_and(|e| e == "lua") {
                                found = true;
                                match LuaPlugin::load(&path) {
                                    Ok(plugin) => {
                                        let meta = plugin.metadata();
                                        println!(
                                            "  {} {} - {}",
                                            meta.name.green(),
                                            format!("v{}", meta.version).dimmed(),
                                            meta.description
                                        );
                                    }
                                    Err(e) => {
                                        let name = path.file_stem()
                                            .map(|s| s.to_string_lossy().to_string())
                                            .unwrap_or_else(|| "unknown".to_string());
                                        println!("  {} {}", name.red(), format!("(error: {})", e).dimmed());
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if !found {
                println!("  No plugins installed.");
                println!();
                println!("  Install plugins with:");
                println!("    {} plugin install <path-to-plugin.lua>", "prog".cyan());
            }
        }

        PluginAction::Install { name, version: _, git } => {
            let source_path = PathBuf::from(&name);

            if git {
                // Git URL installation (future feature)
                eprintln!("{} Git URL installation coming soon.", "⚠️".yellow());
                eprintln!("  For now, download the plugin and use: prog plugin install <path>");
                return Ok(());
            }

            if !source_path.exists() {
                return Err(anyhow!("Plugin file not found: {}", name));
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

        PluginAction::Info { name } => {
            let mut found = false;

            for dir in [&local_plugins, &home_plugins] {
                let plugin_path = dir.join(format!("{}.lua", name));
                if plugin_path.exists() {
                    match LuaPlugin::load(&plugin_path) {
                        Ok(plugin) => {
                            let meta = plugin.metadata();
                            println!("{}", "Plugin Information".bold());
                            println!("{}", "─".repeat(40));
                            println!("  Name:        {}", meta.name.green());
                            println!("  Version:     {}", meta.version);
                            println!("  Author:      {}", meta.author);
                            println!("  Description: {}", meta.description);
                            println!("  Location:    {}", plugin_path.display());
                            println!("  Hooks:       {:?}", meta.hooks.iter().map(|h| format!("{:?}", h)).collect::<Vec<_>>().join(", "));
                            found = true;
                        }
                        Err(e) => {
                            return Err(anyhow!("Failed to load plugin: {}", e));
                        }
                    }
                    break;
                }
            }

            if !found {
                return Err(anyhow!("Plugin '{}' not found", name));
            }
        }

        PluginAction::Update { name: _ } => {
            eprintln!("{} Plugin update requires registry server (coming soon)", "⚠️".yellow());
            eprintln!("  For now, manually download and reinstall plugins.");
        }

        PluginAction::Search { query: _ } => {
            eprintln!("{} Plugin search requires registry server (coming soon)", "⚠️".yellow());
            eprintln!("  Browse available plugins at: https://git.maiwald.work/SSSS/progit-plugins-index");
        }

        PluginAction::Index { action: index_action } => {
            match index_action {
                IndexAction::Update => {
                    eprintln!("{} Index update requires registry server (coming soon)", "⚠️".yellow());
                }
            }
        }
    }

    Ok(())
}

/// Handle hooks CLI commands
fn handle_hooks_command(action: HooksAction, repo_root: &Path) -> Result<()> {
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

/// Convert ProGit Issue to Plugin SDK Issue format
fn convert_issue_to_plugin(issue: &Issue) -> progit_plugin_sdk::prelude::Issue {
    progit_plugin_sdk::prelude::Issue {
        id: issue.id.clone(),
        title: issue.title.clone(),
        description: issue.description.clone(),
        status: issue.status.as_str().to_string(),
        tags: issue.tags.clone(),
        assignee: issue.assignee.clone(),
        effort: Some(issue.effort as u8),
        blocked: issue.blocked,
        created: issue.created.to_rfc3339(),
        updated: issue.updated.to_rfc3339(),
        due: issue.due.map(|d| d.to_rfc3339()),
        metadata: std::collections::HashMap::new(),
    }
}

fn start_tui() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let result = run_app(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Handle errors
    if let Err(err) = result {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>) -> Result<()> {
    let mut app = App::new();

    // Determine paths
    let project_root = find_project_root()?;
    let config_path = project_root.join(paths::config_file());

    // Run migration from KDL to JSON if needed
    match storage::migrate::migrate_kdl_to_json(&project_root) {
        Ok(count) if count > 0 => {
            log::info!("✅ Migrated {} issues from KDL to JSON", count);
        }
        Err(e) => {
            log::warn!("⚠️ Migration failed: {}", e);
        }
        _ => {}
    }

    // Initialize storage engine
    let mut engine = storage::engine::StorageEngine::new(&project_root);
    engine.load()?;

    // Load config & init provider
    if let Ok(config) = storage::config::load_config(&config_path) {
        if let Some(sync_config) = config.sync {
            app.sync_provider_name = Some(sync_config.provider.clone());
            app.sync_provider = Some(sync::create_provider(sync_config));
        }
        // Apply saved theme
        if let Some(theme_name) = config.theme {
            app.theme = match theme_name.as_str() {
                "nord" => tui::Theme::Nord,
                "gruvbox" => tui::Theme::Gruvbox,
                "dracula" => tui::Theme::Dracula,
                "cyberpunk" => tui::Theme::Cyberpunk,
                "vibe" => tui::Theme::Vibe,
                _ => tui::Theme::Nord,
            };
        }

        // Initialize style engine with configured styles
        app.theme_engine = crate::tui::style::ThemeEngine::new(&config.styles);

        // Validate styles and warn if needed
        if let Err(e) = app.theme_engine.validate() {
            app.set_status(format!("Style Config Error: {}", e));
        }
    }

    // Load data from engine
    app.load_issues(engine.issues().to_vec());
    app.load_mrs(engine.mrs().to_vec());
    
    // Detect git repository from current working directory
    let cwd = std::env::current_dir()?;
    app.repo_info = detect_repo(&cwd)?;

    // ─── Plugin Loading ────────────────────────────────────────────────────────
    // Load plugins from plugins/ (repo) and .progit/plugins/ (user-installed)
    {
        use progit_plugin_sdk::prelude::PluginContext;

        let context = PluginContext {
            repo_path: project_root.to_string_lossy().to_string(),
            user: std::env::var("USER").ok(),
            env: std::env::vars().collect(),
            config: std::collections::HashMap::new(),
        };

        let mut plugin_manager = crate::plugins::PluginManager::new(&project_root);

        // Load from repo plugins/ directory
        match plugin_manager.load_all(&context) {
            Ok(count) if count > 0 => {
                log::info!("🔌 Loaded {} repo plugin(s)", count);
            }
            Err(e) => {
                log::warn!("⚠️ Repo plugin loading failed: {}", e);
            }
            _ => {}
        }

        // Also load from .progit/plugins/ (user-installed)
        let user_plugins = project_root.join(".progit").join("plugins");
        if user_plugins.exists() {
            plugin_manager.load_from_dir(&user_plugins, &context);
        }

        app.plugin_manager = Some(plugin_manager);
    }

    // ─── Panopticum Integration ───────────────────────────────────────────────
    app.repo_path = project_root.clone();
    app.is_panopticum_repo = crate::panopticum::is_panopticum_repo(&project_root);

    if app.is_panopticum_repo {
        log::info!("🔱 Panopticum mode activated");
        // Check if panoctl is available (lazy check - don't fail if missing)
        if !crate::panopticum::is_panoctl_available(None) {
            log::warn!("⚠️ panoctl binary not found in PATH");
        }
        // Create event channel for async operations
        let (tx, rx) = crate::panopticum::create_event_channel();
        app.pano_event_tx = Some(tx);
        app.pano_event_rx = Some(rx);
    }

    // Track UI areas for mouse events
    let mut ui_areas = UIAreas::default();

    // ─── Agent Integration ───────────────────────────────────────────────────
    let (tx, rx) = std::sync::mpsc::channel();
    app.agent_event_tx = Some(tx);
    app.agent_event_rx = Some(rx);

    loop {
        // ─── Panopticum Event Polling ─────────────────────────────────────────
        // Check for async panopticum results before rendering
        if let Some(rx) = app.pano_event_rx.take() {
            while let Ok(event) = rx.try_recv() {
                match event {
                    crate::panopticum::PanoEvent::Status(status) => {
                        match &status {
                            crate::panopticum::PanoStatus::OutputLine(line) => {
                                app.pano_output.push(line.clone());
                                // Show last line in status bar
                                if let Some(last) = app.pano_output.last() {
                                    app.set_status(last.clone());
                                }
                            }
                            _ => app.pano_status = status,
                        }
                    }
                    crate::panopticum::PanoEvent::ValidationComplete { success, message } => {
                        if success {
                            app.pano_status =
                                crate::panopticum::PanoStatus::Success(message.clone());
                            app.set_status(format!("✓ {}", message));
                        } else {
                            app.pano_status = crate::panopticum::PanoStatus::Error(message.clone());
                            app.set_status(format!("✗ {}", message));
                        }
                    }
                    crate::panopticum::PanoEvent::PlanComplete { success, output } => {
                        if success {
                            app.pano_status =
                                crate::panopticum::PanoStatus::Success("Plan complete".into());
                            app.set_status("✓ Plan completed successfully");
                        } else {
                            app.pano_status = crate::panopticum::PanoStatus::Error(output.clone());
                            app.set_status(format!("✗ Plan failed"));
                        }
                    }
                    crate::panopticum::PanoEvent::ApplyComplete { success, output } => {
                        if success {
                            app.pano_status =
                                crate::panopticum::PanoStatus::Success("Apply complete".into());
                            app.set_status("✓ Apply completed successfully");
                        } else {
                            app.pano_status = crate::panopticum::PanoStatus::Error(output.clone());
                            app.set_status(format!("✗ Apply failed"));
                        }
                    }
                }
            }
            // Put receiver back
            app.pano_event_rx = Some(rx);
        }

        // ─── Agent Event Polling ──────────────────────────────────────────────
        if let Some(rx) = app.agent_event_rx.take() {
            while let Ok(event) = rx.try_recv() {
                use crate::agent::AgentEvent;
                match event {
                    AgentEvent::Started(id) => {
                         // Update session status to Thinking
                         // This would ideally map id back to a virtual branch
                         // For now we just show a status
                         app.set_status(format!("🤖 Agent started (Session {})", &id[..8]));
                    }
                    AgentEvent::Token(_id, token) => {
                         // Streaming token - in future we append to a buffer
                         // For now, simple indicator
                         // app.set_status(format!("🤖 Typing... {}", token)); // Too noisy
                    }
                    AgentEvent::Completed(id, response) => {
                         app.set_status("🤖 Agent finished! Applying changes...");
                         
                         if let Some(manager) = &mut app.vbranch_manager {
                             use crate::agent::ops::apply_agent_patch;
                             match apply_agent_patch(manager, &id, &response) {
                                 Ok(count) => {
                                     app.set_status(format!("✅ Agent applied {} new hunk(s)", count));
                                 }
                                 Err(e) => {
                                     log::error!("Agent apply error: {}", e);
                                     app.set_status(format!("❌ Failed to apply agent patch: {}", e));
                                 }
                             }
                         }
                    }
                    AgentEvent::Error(_id, err) => {
                         app.set_status(format!("⚠️ Agent error: {}", err));
                    }
                }
            }
            app.agent_event_rx = Some(rx);
        }

        // Draw
        terminal.draw(|frame| {
            ui_areas = render(frame, &mut app);
        })?;

        // Handle input
        if event::poll(std::time::Duration::from_millis(100))? {
            let action = match event::read()? {
                Event::Key(key) => handle_key(&mut app, key),
                Event::Mouse(mouse) => handle_mouse(&mut app, mouse, &ui_areas),
                _ => KeyAction::None,
            };

            match action {
                KeyAction::Quit => break,
                KeyAction::Save => {
                    // Sync app issues to engine and save
                    *engine.issues_mut() = app.issues.clone();
                    engine.save()?;
                }
                KeyAction::SaveTheme => {
                    // Save theme preference to config
                    let theme_name = match app.theme {
                        tui::Theme::Nord => "nord",
                        tui::Theme::Gruvbox => "gruvbox",
                        tui::Theme::Dracula => "dracula",
                        tui::Theme::Cyberpunk => "cyberpunk",
                        tui::Theme::Vibe => "vibe",
                    };
                    let _ = storage::save_theme(&config_path, theme_name);
                    app.set_status(format!("Theme: {}", theme_name));
                }
                KeyAction::CreateIssue(status) => {
                    // Create a new issue with optional status
                    let mut new_issue = Issue::new("New Issue");
                    if let Some(s) = status {
                        new_issue.status = s;
                    }
                    engine.upsert(new_issue.clone())?;
                    app.load_issues(engine.issues().to_vec());
                    app.set_status("Created new issue");

                    // Trigger plugin hook
                    if let Some(ref mut pm) = app.plugin_manager {
                        let plugin_issue = convert_issue_to_plugin(&new_issue);
                        pm.on_issue_created(&plugin_issue);
                    }
                }
                KeyAction::DeleteIssue => {
                    // Delete the selected issue based on view mode
                    let issue_id = match app.view_mode {
                        tui::ViewMode::Dashboard => None,
                        tui::ViewMode::List => app.selected_issue().map(|i| i.id.clone()),
                        tui::ViewMode::Kanban => app.kanban_selected_issue().map(|i| i.id.clone()),
                        tui::ViewMode::Diff => None,
                        tui::ViewMode::MRList => None,
                        tui::ViewMode::Blame => None,
                        tui::ViewMode::Lanes => None,
                    };

                    if let Some(id) = issue_id {
                        if engine.delete(&id)? {
                            app.load_issues(engine.issues().to_vec());
                            app.set_status("Issue deleted");

                            // Trigger plugin hook
                            if let Some(ref mut pm) = app.plugin_manager {
                                pm.on_issue_deleted(&id);
                            }
                        } else {
                            app.set_status("Failed to delete issue");
                        }
                    }
                }
                KeyAction::Sync => {
                    // Temporarily take provider to avoid borrow conflict
                    if let Some(provider) = app.sync_provider.take() {
                        let provider_name = app.sync_provider_name.as_deref().unwrap_or("remote");
                        app.set_status(format!("Syncing with {}...", provider_name));
                        terminal.draw(|frame| {
                            ui_areas = render(frame, &mut app);
                        })?;

                        // 1. PUSH
                        if let Err(e) = provider
                            .login()
                            .and_then(|_| provider.push(&mut app.issues))
                        {
                            app.set_status(format!("Push failed: {}", e));
                        } else {
                            // Persist links after push
                            *engine.issues_mut() = app.issues.clone();
                            if let Err(e) = engine.save() {
                                app.set_status(format!("Save failed: {}", e));
                            } else {
                                // 2. DELETE MISSING
                                let _ = provider.delete_missing(&app.issues);

                                // 3. PULL ISSUES
                                match provider.pull() {
                                    Ok(remote_issues) => {
                                        let provider_name =
                                            app.sync_provider_name.as_deref().unwrap_or("gitlab");
                                        let merged = sync::merge_issues(
                                            &app.issues,
                                            remote_issues,
                                            provider_name,
                                        );
                                        app.load_issues(merged.clone());
                                        *engine.issues_mut() = merged;
                                    }
                                    Err(e) => app.set_status(format!("Issues pull failed: {}", e)),
                                }

                                // 4. PULL MRS
                                match provider.list_mrs() {
                                    Ok(remote_mrs) => {
                                        let provider_name =
                                            app.sync_provider_name.as_deref().unwrap_or("gitlab");
                                        let merged = sync::merge_mrs(
                                            &app.mr_list,
                                            remote_mrs,
                                            provider_name,
                                        );
                                        app.load_mrs(merged.clone());
                                        *engine.mrs_mut() = merged;
                                        app.set_status("Sync Complete (Issues & MRs)!");
                                    }
                                    Err(e) => app.set_status(format!("MR pull failed: {}", e)),
                                }

                                // Final Save
                                if let Err(e) = engine.save() {
                                    app.set_status(format!("Save failed: {}", e));
                                }
                            }
                        }
                        // Put provider back
                        app.sync_provider = Some(provider);
                    } else {
                        app.set_status("No sync provider configured.");
                    }
                }
                KeyAction::SwitchBranch(branch) => {
                    match crate::git::switch_branch(&cwd, &branch) {
                        Ok(_) => {
                            app.set_status(format!("Switched to branch: {}", branch));
                            // Refresh repo info
                            app.repo_info = detect_repo(&cwd)?;

                            // Reload issues from disk/engine
                            if let Err(e) = engine.load() {
                                app.set_status(format!("Reload failed: {}", e));
                            } else {
                                app.load_issues(engine.issues().to_vec());
                            }
                        }
                        Err(e) => app.set_status(format!("Failed to switch: {}", e)),
                    }
                }
                KeyAction::CreateBranch => {
                    // For now, simpler prompts. ideally we use an input box?
                    // MVP: Just auto-generate a name or prompt via "Edit" mode?
                    // We don't have a generic "InputBox" widget yet for random strings.
                    // We only have `edit_buffer` used for issues.
                    // Let's reuse `input_mode = InputMode::Edit` (legacy) or hijack search?
                    // Hack: Create "branch-TIMESTAMP" for MVP or ask user to implement Input Box properly later.
                    // Actually, let's use the USER REQUEST context: "creating a new one".
                    // I will stick to "create-feature" for now to check if it works.
                    // Wait, `Edit` mode in input.rs says "Legacy - redirect to detail view".
                    // Let's implement a real input dialog next time.
                    // For now, let's create "new-branch-<timestamp>"

                    let new_name = format!("branch-{}", chrono::Utc::now().timestamp());
                    match crate::git::create_branch(&cwd, &new_name) {
                        Ok(_) => {
                            app.set_status(format!("Created {}", new_name));
                            app.repo_info = detect_repo(&cwd)?;
                        }
                        Err(e) => app.set_status(format!("Failed to create: {}", e)),
                    }
                }
                KeyAction::CreateBranchNamed(name) => {
                    match crate::git::create_branch(&cwd, &name) {
                        Ok(_) => {
                            app.set_status(format!("Created & switched to: {}", name));
                            app.repo_info = detect_repo(&cwd)?;
                        }
                        Err(e) => app.set_status(format!("Failed to create '{}': {}", name, e)),
                    }
                }
                KeyAction::DeleteBranch(name) => match crate::git::delete_branch(&cwd, &name) {
                    Ok(_) => {
                        app.set_status(format!("Deleted branch: {}", name));
                        app.repo_info = detect_repo(&cwd)?;
                    }
                    Err(e) => app.set_status(format!("Failed to delete '{}': {}", name, e)),
                },
                KeyAction::Refresh | KeyAction::None => {}
                KeyAction::ToggleDebug => {
                    app.show_debug_console = !app.show_debug_console;
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

/// Save all issues (for drag-drop operations)
fn save_all_issues(
    issues: &[Issue],
    kdl_dir: &std::path::Path,
    cache_path: &std::path::Path,
) -> Result<()> {
    for issue in issues {
        save_issue(issue, kdl_dir, cache_path)?;
    }
    Ok(())
}

/// Find the project root (directory containing .git or .project)
fn find_project_root() -> Result<PathBuf> {
    let current = std::env::current_dir()?;

    // Walk up looking for .git (repo root), .project (existing setup), or PANOPTICUM.kdl (infra repo)
    let mut path = current.as_path();
    loop {
        if path.join(".git").exists()
            || path.join(storage::paths::PROJECT_DIR).exists()
            || path.join(".projects").exists()
            || path.join("PANOPTICUM.kdl").exists()
        {
            return Ok(path.to_path_buf());
        }
        match path.parent() {
            Some(parent) => path = parent,
            None => break,
        }
    }

    // Fall back to current directory
    Ok(current)
}

/// Initialize the workspace (.project and .progit dirs)
/// Initialize the workspace (.project and .progit dirs)
fn initialize_workspace(root: &Path) -> Result<()> {
    let project_dir = root.join(storage::paths::PROJECT_DIR);
    let local_dir = root.join(storage::paths::LOCAL_DIR);

    // 1. Create directories
    if !project_dir.exists() {
        println!("✨ Initializing .project/ ...");
        std::fs::create_dir(&project_dir)?;
        let issues_dir = project_dir.join("issues");
        if !issues_dir.exists() {
            std::fs::create_dir(&issues_dir)?;
        }

        // Try to detect git remote for config
        let mut sync_config = String::new();
        let mut provider_msg = "Local Mode".to_string();

        if let Ok(Some(origin)) = crate::git::get_origin_url(root) {
            println!("   🔍 Detected git remote: {}", origin);
            if let Some((h, o, r)) = crate::git::parse_git_url(&origin) {
                // Heuristic detection
                let provider = if h.contains("gitlab") {
                    "gitlab"
                } else {
                    "forgejo"
                };

                sync_config = format!(
                    r#"sync {{
    provider "{}"
    url "{}"
    owner "{}"
    repo "{}"
}}
"#,
                    provider, h, o, r
                );
                provider_msg = format!("Provider: {}", provider);
            }
        }

        if sync_config.is_empty() {
            println!("   ℹ️ No compatible forge remote detected. Initializing in Local Mode.");
            sync_config = "// No sync configuration (Local Mode)\n// To enable sync, add a 'sync' block with provider, url, owner, and repo\n".to_string();
        }

        let config_content = format!(
            r#"// ProGit Configuration
// This file is auto-generated but safe to edit manually

config {{
    // Sprint duration in days
    sprint-duration-days 14
    
    // Team members (for assignee dropdown/autocomplete)
    team {{
        // Add your team members here, e.g.:
        // - "alice"
        // - "bob"
    }}
    
    // Default effort estimate for new issues (1-5)
    default-effort 3
}}

{0}
// Theme: nord, gruvbox, dracula, cyberpunk
theme "nord"
"#,
            sync_config
        );

        std::fs::write(project_dir.join("config.kdl"), config_content)?;
        println!("   Created config.kdl ({})", provider_msg);
    }

    // Ensure issues dir exists if project dir was already there
    let issues_dir = project_dir.join("issues");
    if !issues_dir.exists() {
        std::fs::create_dir(&issues_dir)?;
    }

    if !local_dir.exists() {
        std::fs::create_dir(&local_dir)?;
    }

    // 2. Update .gitignore
    let gitignore = root.join(".gitignore");
    if gitignore.exists() {
        let content = std::fs::read_to_string(&gitignore)?;
        if !content.contains(".progit") {
            println!("🔒 Adding .progit to .gitignore...");
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .open(&gitignore)?;
            use std::io::Write;
            writeln!(file, "\n# ProGit local state")?;
            writeln!(file, ".progit/")?;
        }
    }

    Ok(())
}

fn auto_configure(sync_config: &mut storage::config::SyncConfig, cwd: &std::path::Path) {
    if let Ok(Some(remote_url)) = crate::git::get_remote_url(cwd, "origin") {
        // Only trigger change detection if we can successfully parse the remote URL
        if let Some((base, _, _)) = crate::git::parse_git_url(&remote_url) {
            // Compare BASE URLs (e.g. "gitlab.com" vs "gitlab.com"), not full paths
            // This fixes the bug where "https://gitlab.com" != "git@gitlab.com:user/repo.git" triggered false detection
            let current_base = sync_config.url.trim_end_matches('/');
            let detected_base = base.trim_end_matches('/');

            if current_base != detected_base {
                // Detect Provider Type
                if remote_url.contains("gitlab") {
                    sync_config.provider = "gitlab".to_string();
                } else if remote_url.contains("gitea")
                    || remote_url.contains("forgejo")
                    || remote_url.contains("codeberg")
                    || base.contains("maiwald.work")
                {
                    sync_config.provider = "forgejo".to_string();
                }

                // Parse URL components properly
                if let Some((base, owner, repo)) = crate::git::parse_git_url(&remote_url) {
                    println!(
                        "{} Detected remote change: {} -> {}",
                        "⚡".yellow(),
                        sync_config.url,
                        base
                    );
                    sync_config.url = base;
                    sync_config.owner = owner.clone();
                    sync_config.repo = repo.clone();
                    println!(
                        "{} Auto-configured for {}/{} ({})",
                        "🔧".yellow(),
                        owner,
                        repo,
                        sync_config.provider
                    );
                } else {
                    // Fallback manual parsing if parse_git_url fails (e.g. non-standard URL)
                    sync_config.url = remote_url.clone();
                    let clean_remote = remote_url.trim_end_matches(".git").trim_end_matches('/');
                    let parts: Vec<&str> = clean_remote.split(&['/', ':'][..]).collect();
                    if parts.len() >= 2 {
                        let repo = parts.last().unwrap();
                        let owner = parts.get(parts.len() - 2).unwrap();
                        sync_config.owner = owner.to_string();
                        sync_config.repo = repo.to_string();
                    }
                }
            }
        }
    }
}
