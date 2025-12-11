//! ProjectsTUI - Lean Git Issue Tracker
//!
//! Terminal cockpit f#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

mod git;
mod issue;
mod mr;
mod plugins;
mod fuzzy;
mod storage;
mod sync;
mod tui;
mod command;

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
use crate::tui::{handle_key, handle_mouse, render, App, KeyAction, UIAreas};
use crate::sync::SyncProvider;
use anyhow::{Context, anyhow};

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
}

#[derive(Subcommand)]
enum MrAction {
    /// List open merge requests (requires sync config)
    List,
    /// Create a new merge request (interactive or from args)
    Create {
        /// Target branch (default: main/master)
        #[arg(short, long)]
        target: Option<String>,
        /// Title (default: from last commit or branch name)
        #[arg(short, long)]
        title: Option<String>,
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
        // If no .project exists, we require a git repository
        if crate::git::detect_repo(&project_root)?.is_none() {
            println!("{} No git repository found.", "❌".red());
            println!("   ProGit requires a git repository to initialize.");
            println!("   Please run 'git init' first.");
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

            let sync_config = config.sync.context(
                "No sync configuration found. Please add a 'sync' block to .projects/config.kdl"
            )?;

            // Choose provider
            let provider = sync::create_provider(sync_config.clone());

            // Sync command handling
            match action {
                Some(SyncAction::Push) => {
                    // Load all issues
                    let kdl_dir = project_root.join(paths::issues_dir());
                    let cache_path = project_root.join(paths::cache_file());
                    let mut issues = load_issues(&kdl_dir, &cache_path)?;
                    
                    println!("{} Authenticating with {}...", "🔄".blue(), sync_config.provider);
                    provider.login()?;
                    
                    println!("{} Pushing issues...", "🔄".blue());
                    provider.push(&mut issues)?;
                    
                    // Save issues back to persist new remote IDs
                    println!("{} Saving {} issues with remote links...", "💾".green(), issues.len());
                    for issue in &issues {
                        save_issue(issue, &kdl_dir, &cache_path)?;
                    }
                    
                    // Delete remote issues not in local
                    let deleted = provider.delete_missing(&issues)?;
                    if deleted > 0 {
                        println!("{} Deleted {} remote issues.", "🗑️".red(), deleted);
                    }
                    
                    println!("{} Push complete - Links saved.", "✅".green());
                }
                Some(SyncAction::Pull) => {
                    let kdl_dir = project_root.join(paths::issues_dir());
                    let cache_path = project_root.join(paths::cache_file());
                    
                    // Load local issues for deduplication matching
                    let local_issues = load_issues(&kdl_dir, &cache_path)?;
                    
                    println!("{} Authenticating with {}...", "🔄".blue(), sync_config.provider);
                    provider.login()?;
                    
                    println!("{} Pulling issues...", "🔄".blue());
                    let remote_issues = provider.pull()?;
                    
                    println!("{} Pulled {} issues. Merging...", "📥".blue(), remote_issues.len());
                    
                    let provider_name = &sync_config.provider;
                    println!("{} Using provider '{}' for basic deduplication.", "🔎".cyan(), provider_name);
                    
                    let merged_issues = sync::merge_issues(&local_issues, remote_issues, provider_name);
                    
                    // Save merged set
                    println!("{} Saving {} merged issues...", "💾".green(), merged_issues.len());
                    for issue in merged_issues {
                        save_issue(&issue, &kdl_dir, &cache_path)?;
                    }
                    println!("{} Pull complete.", "✅".green());
                }
                None => {}
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
                    println!("{} Switched to branch {}", "✅".green(), name.green().bold());
                }
                Some(BranchAction::Create { name }) => {
                    crate::git::create_branch(&cwd, &name)?;
                    println!("{} Created and switched to branch {}", "✅".green(), name.green().bold());
                }
                Some(BranchAction::Delete { name }) => {
                    crate::git::delete_branch(&cwd, &name)?;
                    println!("{} Deleted branch {}", "🗑️".red(), name.red());
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

            let sync_config = config.sync.context(
                "No sync configuration found. MR commands require a configured provider."
            )?;
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
                            println!("  {} {} {}", 
                                format!("!{}", mr.id).cyan(), 
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

                    println!("{} Creating MR: {} -> {}", "🔄".blue(), source_branch.cyan(), target_branch.cyan());
                    
                    let mr = crate::mr::MergeRequest::new(&source_branch, &target_branch, &mr_title);
                    match provider.create_mr(&mr) {
                        Ok(id) => println!("{} Created MR !{}", "✅".green(), id),
                        Err(e) => println!("{} Failed to create MR: {}", "❌".red(), e),
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
            let kdl_dir = project_root.join(paths::issues_dir());
            let cache_path = project_root.join(paths::cache_file());
            
            let mut issues = load_issues(&kdl_dir, &cache_path)?;
            
            // Find issue by full or short ID
            if let Some(issue) = issues.iter_mut().find(|i| i.id == id || i.id.starts_with(&id)) {
                issue.blocked = !issue.blocked;
                issue.updated = chrono::Utc::now();
                let blocked_str = if issue.blocked { "BLOCKED".red().bold() } else { "UNBLOCKED".green().bold() };
                println!("{} Issue {} marked as {}", "🔥".yellow(), issue.short_id().bold(), blocked_str);
                save_issue(issue, &kdl_dir, &cache_path)?;
            } else {
                return Err(anyhow!("Issue '{}' not found", id));
            }
        }
        Some(Commands::Due { id, date }) => {
            let project_root = find_project_root()?;
            let kdl_dir = project_root.join(paths::issues_dir());
            let cache_path = project_root.join(paths::cache_file());
            
            let mut issues = load_issues(&kdl_dir, &cache_path)?;
            
            // Find issue by full or short ID
            if let Some(issue) = issues.iter_mut().find(|i| i.id == id || i.id.starts_with(&id)) {
                if date.to_lowercase() == "clear" {
                    issue.due = None;
                    println!("{} Due date cleared for issue {}", "⏰".green(), issue.short_id().bold());
                } else {
                    // Parse date (YYYY-MM-DD format)
                    let parsed_date = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")
                        .context("Invalid date format. Use YYYY-MM-DD")?;
                    let due_datetime = parsed_date.and_hms_opt(23, 59, 59)
                        .context("Invalid time")?;
                    issue.due = Some(chrono::DateTime::from_naive_utc_and_offset(due_datetime, chrono::Utc));
                    println!("{} Due date set to {} for issue {}", "⏰".green(), date.cyan(), issue.short_id().bold());
                }
                issue.updated = chrono::Utc::now();
                save_issue(issue, &kdl_dir, &cache_path)?;
            } else {
                return Err(anyhow!("Issue '{}' not found", id));
            }
        }
        None => {
            // No command - run TUI
            start_tui()?;
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

    // Load issues from engine
    app.load_issues(engine.issues().to_vec());

    // Detect git repository from current working directory
    let cwd = std::env::current_dir()?;
    app.repo_info = detect_repo(&cwd)?;

    // Initialize plugin manager and load plugins
    let mut plugin_manager = crate::plugins::PluginManager::new(&project_root);
    let plugin_context = progit_plugin_sdk::prelude::PluginContext {
        repo_path: project_root.to_string_lossy().to_string(),
        user: std::env::var("USER").ok(),
        env: std::env::vars().collect(),
        config: std::collections::HashMap::new(),
    };
    
    match plugin_manager.load_all(&plugin_context) {
        Ok(count) if count > 0 => {
            log::info!("✅ Loaded {} plugin(s)", count);
            app.set_status(format!("Loaded {} plugin(s)", count));
        }
        Ok(_) => {
            log::info!("No plugins found");
        }
        Err(e) => {
            log::warn!("⚠️ Plugin loading failed: {}", e);
        }
    }
    
    app.plugin_manager = Some(plugin_manager);

    // Track UI areas for mouse events
    let mut ui_areas = UIAreas::default();

    loop {
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
                        tui::ViewMode::List => app.selected_issue().map(|i| i.id.clone()),
                        tui::ViewMode::Kanban => app.kanban_selected_issue().map(|i| i.id.clone()),
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
                        if let Err(e) = provider.login().and_then(|_| provider.push(&mut app.issues)) {
                            app.set_status(format!("Push failed: {}", e));
                        } else {
                            // Persist links after push
                            *engine.issues_mut() = app.issues.clone();
                            if let Err(e) = engine.save() {
                                app.set_status(format!("Save failed: {}", e));
                            } else {
                                // 2. DELETE MISSING
                                let _ = provider.delete_missing(&app.issues);

                                // 3. PULL
                                match provider.pull() {
                                    Ok(remote_issues) => {
                                        // Merge using actual provider name
                                        let provider_name = app.sync_provider_name.as_deref().unwrap_or("gitlab");
                                        let merged = sync::merge_issues(&app.issues, remote_issues, provider_name);
                                        app.load_issues(merged.clone());
                                        
                                        *engine.issues_mut() = merged;
                                        if let Err(e) = engine.save() {
                                            app.set_status(format!("Save failed: {}", e));
                                        } else {
                                            app.set_status("Sync Complete!");
                                        }
                                    }
                                    Err(e) => app.set_status(format!("Pull failed: {}", e)),
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
                         },
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
                        },
                         Err(e) => app.set_status(format!("Failed to create: {}", e)),
                    }
                }
                KeyAction::CreateBranchNamed(name) => {
                    match crate::git::create_branch(&cwd, &name) {
                        Ok(_) => {
                            app.set_status(format!("Created & switched to: {}", name));
                            app.repo_info = detect_repo(&cwd)?;
                        },
                        Err(e) => app.set_status(format!("Failed to create '{}': {}", name, e)),
                    }
                }
                KeyAction::DeleteBranch(name) => {
                    match crate::git::delete_branch(&cwd, &name) {
                        Ok(_) => {
                            app.set_status(format!("Deleted branch: {}", name));
                            app.repo_info = detect_repo(&cwd)?;
                        },
                        Err(e) => app.set_status(format!("Failed to delete '{}': {}", name, e)),
                    }
                }
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

    // Walk up looking for .git (repo root) or .project (existing setup)
    let mut path = current.as_path();
    loop {
        if path.join(".git").exists() || path.join(storage::paths::PROJECT_DIR).exists() || path.join(".projects").exists() {
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
fn initialize_workspace(root: &Path) -> Result<()> {
    let project_dir = root.join(storage::paths::PROJECT_DIR);
    let local_dir = root.join(storage::paths::LOCAL_DIR);

    // 1. Create directories
    if !project_dir.exists() {
        println!("✨ Initializing .project/ ...");
        std::fs::create_dir(&project_dir)?;
        std::fs::create_dir(project_dir.join("issues"))?;
        
        // Try to detect git remote for config
        let mut sync_config = String::new();
        let mut provider_msg = "Local Mode".to_string();

        if let Ok(Some(origin)) = crate::git::get_origin_url(root) {
            println!("   🔍 Detected git remote: {}", origin);
            if let Some((h, o, r)) = crate::git::parse_git_url(&origin) {
                 // Heuristic detection
                 let provider = if h.contains("gitlab") { "gitlab" } else { "forgejo" };
                 
                 sync_config = format!(r#"sync {{
    provider "{}"
    url "{}"
    owner "{}"
    repo "{}"
}}
"#, provider, h, o, r);
                 provider_msg = format!("Provider: {}", provider);
            }
        }
        
        if sync_config.is_empty() {
            println!("   ℹ️ No compatible forge remote detected. Initializing in Local Mode.");
            sync_config = "// No sync configuration (Local Mode)\n// To enable sync, add a 'sync' block with provider, url, owner, and repo\n".to_string();
        }

        let config_content = format!(r#"// ProGit Configuration
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
"#, sync_config);

        std::fs::write(project_dir.join("config.kdl"), config_content)?;
        println!("   Created config.kdl ({})", provider_msg);
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
