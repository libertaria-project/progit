//! ProGit - Git-First Project Management
//!
//! Terminal cockpit for developers: virtual branches, Kanban, AI agent, forge sync.
//
// Dead code is expected: many modules expose future API surface not yet wired to TUI/CLI.
// Unused imports and variables are NOT suppressed — those are real hygiene issues.
#![allow(dead_code)]

mod cli;
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
mod review;
mod review_sync;
mod runner;
mod storage;
mod sync;
mod tui;
mod virtual_branch;
mod agent;
mod workspace;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use colored::*;

use crate::git::detect_repo;
use crate::storage::paths;

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
pub(crate) enum PluginAction {
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
    /// Scaffold a new plugin in plugins/<name>/
    New {
        /// Plugin name (lowercase, kebab-case recommended)
        name: String,
        /// Plugin author (defaults to git user.name or "Anonymous")
        #[arg(long)]
        author: Option<String>,
    },
}

#[derive(Subcommand)]
pub(crate) enum IndexAction {
    /// Update the local plugin index from remote
    Update,
}

#[derive(Subcommand)]
pub(crate) enum HooksAction {
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
    // Initialize Logger — failure is non-fatal, continue without logging
    let _ = tui_logger::init_logger(log::LevelFilter::Trace);
    tui_logger::set_default_level(log::LevelFilter::Info);

    // 1. Detect Root
    let project_root = workspace::find_project_root()?;

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
    workspace::initialize_workspace(&project_root)?;

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Sync { action }) => {
            // Load config
            let project_root = workspace::find_project_root()?;
            let config_path = project_root.join(paths::config_file());
            let config = storage::config::load_config(&config_path)?;

            let mut sync_config = config.sync.context(
                "No sync configuration found. Please add a 'sync' block to .projects/config.kdl",
            )?;

            // Auto-configure based on remote
            let cwd = std::env::current_dir()?;
            workspace::auto_configure(&mut sync_config, &cwd);

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
            let project_root = workspace::find_project_root()?;
            let config_path = project_root.join(paths::config_file());
            let config = storage::config::load_config(&config_path)?;
            let cwd = std::env::current_dir()?;
            let repo_info = detect_repo(&cwd)?.context("Not a git repository")?;

            let mut sync_config = config.sync.context(
                "No sync configuration found. MR commands require a configured provider.",
            )?;

            // Auto-configure based on remote
            workspace::auto_configure(&mut sync_config, &cwd);

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
            storage::cleanup_duplicates(&workspace::find_project_root()?)?;
            println!("{} Cleanup complete.", "✅".green());
        }
        Some(Commands::Block { id }) => {
            let project_root = workspace::find_project_root()?;
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
            let project_root = workspace::find_project_root()?;
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
            cli::handle_plugin_command(action)?;
        }
        Some(Commands::Hooks { action }) => {
            cli::handle_hooks_command(action, &project_root)?;
        }
        Some(Commands::RebaseEditor { path }) => {
            crate::rebase::run(&path)?;
        }
        None => {
            // No command - run TUI
            runner::start_tui()?;
        }
    }

    Ok(())
}
