//! ProGit - Git-First Project Management
//!
//! Terminal cockpit for developers: virtual branches, Kanban, AI agent, forge sync.
//
// Dead code is expected: many modules expose future API surface not yet wired to TUI/CLI.
// Unused imports and variables are NOT suppressed — those are real hygiene issues.
#![allow(dead_code)]

mod agent;
mod cli;
mod command;
mod diff;
mod fuzzy;
mod git;
mod hooks;
mod issue;
mod marketplace;
mod mr;
mod panopticum;
mod plugins;
mod project_contract;
mod project_view;
mod rebase;
mod remote;
mod review;
mod review_sync;
mod runner;
mod sober;
mod storage;
mod sync;
mod tui;
mod virtual_branch;
mod workspace;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use colored::*;

use crate::git::detect_repo;
use crate::marketplace::cli::{handle_trust_command, TrustAction};
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
    /// Manage trusted publisher keys (Hinge security)
    Trust {
        #[command(subcommand)]
        action: TrustAction,
    },
    /// Validate repository-owned project metadata
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
    /// Inspect configured Git remotes and repository contract readiness
    Remote {
        #[command(subcommand)]
        action: RemoteAction,
    },
    /// Run Sober repository-governance checks from ProGit
    Sober {
        #[command(subcommand)]
        action: SoberAction,
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
    /// Clone a repository from a progit-forged daemon into a local backend.
    ///
    /// Demonstrates the GitBackend trait abstraction: source and destination
    /// are both implementations of the same trait, so the clone code path
    /// is identical regardless of where the bytes live.
    #[cfg(feature = "forge-backend")]
    Clone {
        /// Daemon endpoint URL, e.g. `http://127.0.0.1:7421`
        endpoint: String,
        /// Repository name on the daemon
        repo: String,
        /// Local destination directory (created if missing). Default: current directory.
        #[arg(default_value = ".")]
        dest: std::path::PathBuf,
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
    /// Verify plugin integrity (Hinge signature verification)
    Verify {
        /// Plugin name to verify
        name: String,
    },
    /// Run an installed plugin command
    Run {
        /// Plugin command namespace, e.g. sober
        command: String,
        /// Arguments passed through to the plugin
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Dispatch unknown `prog plugin <command>` forms to installed plugins
    #[command(external_subcommand)]
    External(Vec<String>),
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
    /// Validate a commit message or branch name
    Validate {
        /// Type to validate: "commit-msg" or "branch" (auto-detected from content if omitted)
        #[arg(long, default_value = "auto")]
        hook_type: String,
        /// The value to validate (commit message or branch name)
        value: Option<String>,
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
enum ProjectAction {
    /// Validate the `.project/` repository contract
    Validate,
    /// Render repository-owned wiki pages
    Wiki,
    /// List repository-owned issue files
    Issues,
}

#[derive(Subcommand)]
enum RemoteAction {
    /// Check remote reachability and `.project/` contract readiness
    Doctor {
        /// Do not run the dry-run push probe
        #[arg(long)]
        skip_push: bool,
    },
}

#[derive(Subcommand)]
enum SoberAction {
    /// Run `sober doctor`
    Doctor {
        /// Print machine-readable JSON
        #[arg(long)]
        json: bool,
        /// Run online forge checks when token/config allows it
        #[arg(long)]
        online: bool,
    },
    /// Run deterministic `sober preflight`
    Preflight {
        /// Base ref to diff against
        #[arg(long, default_value = "HEAD")]
        base: String,
        /// Print machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Run `sober hygiene check`
    Hygiene {
        /// Hygiene profile
        #[arg(long, default_value = "standard")]
        profile: String,
        /// Print machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Preview a Sober review prompt without calling a model
    ReviewPreview {
        /// Base ref to diff against
        #[arg(long, default_value = "HEAD")]
        base: String,
        /// Provider override, e.g. kimi-coding
        #[arg(long)]
        provider: Option<String>,
        /// Model override, e.g. kimi-k2.6
        #[arg(long)]
        model: Option<String>,
        /// Reviewer profile
        #[arg(long, default_value = "security")]
        reviewer: String,
        /// Review objective
        #[arg(long, default_value = "security")]
        objective: String,
        /// Print machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Manage Sober git hooks
    Hooks {
        #[command(subcommand)]
        action: SoberHooksAction,
    },
    /// Refresh the Sober index
    Index {
        /// Index all tracked files and recent commits
        #[arg(long, conflicts_with = "changed")]
        all: bool,
        /// Index changed files and recent commits
        #[arg(long, conflicts_with = "all")]
        changed: bool,
        /// Print machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Run `sober forge doctor`
    ForgeDoctor {
        /// Print machine-readable JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum SoberHooksAction {
    /// Show Sober hook status
    Status {
        /// Optional hook name: pre-commit or pre-push
        hook: Option<SoberHook>,
        /// Print machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Install Sober managed hooks; with no hook, installs pre-commit and pre-push
    Install {
        /// Optional hook name: pre-commit or pre-push
        hook: Option<SoberHook>,
        /// Print machine-readable JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum SoberHook {
    PreCommit,
    PrePush,
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
    /// Review operations on a merge request
    Review {
        #[command(subcommand)]
        action: ReviewAction,
    },
    /// Reject a merge request (close without merging)
    Reject {
        /// MR number to reject
        id: u64,
    },
}

#[derive(Subcommand)]
enum ReviewAction {
    /// Push local line-level review comments to the configured forge
    Push {
        /// MR number on the remote forge
        mr_id: u64,
    },
}

#[derive(Subcommand)]
enum SyncAction {
    /// Push local issues to remote forge
    Push,
    /// Pull remote forge issues to local
    Pull,
}

/// Push the most-recently-updated local review's comments to the
/// configured forge for the given remote MR id.
///
/// Picks the latest review by `updated_at` so users don't have to remember
/// review IDs — this matches the "I just wrote my comments, now ship them"
/// mental model. If multi-review semantics matter later, add a `--review-id`
/// flag.
fn run_review_push(provider: &dyn crate::sync::SyncProvider, mr_id: u64) -> Result<()> {
    let project_root = workspace::find_project_root()?;
    let storage = crate::review::ReviewStorage::new(&project_root);

    let mut reviews = storage.list().context("Failed to list local reviews")?;
    if reviews.is_empty() {
        return Err(anyhow!(
            "No local reviews found. Enter review mode (`:review <file>` in TUI) and add comments first."
        ));
    }

    reviews.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    let mut review = reviews.into_iter().next().unwrap();

    let total = review.comments.len();
    if total == 0 {
        println!("{} Latest review has no comments to push.", "ℹ️".cyan());
        return Ok(());
    }

    println!(
        "{} Pushing review {} ({} local comment{}) to MR !{}...",
        "🔄".blue(),
        review.id,
        total,
        if total == 1 { "" } else { "s" },
        mr_id
    );

    let pushed = provider
        .push_review_comments(&project_root, mr_id, &review.clone(), &mut review.comments)
        .context("forge refused review-comment push")?;

    // Persist the (possibly mutated) review with new external_ids.
    storage
        .save(&review)
        .context("Failed to save review after push")?;

    let already_synced = review
        .comments
        .iter()
        .filter(|c| !c.external_ids.is_empty())
        .count()
        - pushed;
    let skipped = total.saturating_sub(pushed + already_synced);

    println!(
        "{} Synced {} new (already-synced: {}, skipped: {}) of {} on MR !{}",
        "✅".green(),
        pushed,
        already_synced,
        skipped,
        total,
        mr_id
    );
    Ok(())
}

fn run_project_validate(project_root: &std::path::Path) -> Result<bool> {
    let report = project_contract::validate_project(project_root)?;

    println!(
        "{} Project contract: {}",
        "==>".blue().bold(),
        project_root.display().to_string().dimmed()
    );

    for error in &report.errors {
        print_validation_message("ERROR".red().bold(), error);
    }

    for warning in &report.warnings {
        print_validation_message("WARN".yellow().bold(), warning);
    }

    if report.is_valid() {
        println!(
            "{} valid ({} checks, {} warnings)",
            "OK".green().bold(),
            report.checks_passed,
            report.warnings.len()
        );
    } else {
        println!(
            "{} invalid ({} errors, {} warnings, {} checks passed)",
            "FAIL".red().bold(),
            report.errors.len(),
            report.warnings.len(),
            report.checks_passed
        );
    }

    Ok(report.is_valid())
}

fn run_project_action(project_root: &std::path::Path, action: ProjectAction) -> Result<bool> {
    match action {
        ProjectAction::Validate => run_project_validate(project_root),
        ProjectAction::Wiki => {
            run_project_wiki(project_root)?;
            Ok(true)
        }
        ProjectAction::Issues => {
            run_project_issues(project_root)?;
            Ok(true)
        }
    }
}

fn run_project_wiki(project_root: &std::path::Path) -> Result<()> {
    let view = project_view::load_project_wiki(project_root)?;

    println!(
        "{} Project wiki: {}",
        "==>".blue().bold(),
        project_root.display().to_string().dimmed()
    );
    println!("root: {}", view.root.display().to_string().cyan());

    for page in view.pages {
        let required = if page.required {
            "required"
        } else {
            "optional"
        };
        println!();
        println!(
            "{} {} [{}] {}",
            "==>".blue().bold(),
            page.name.cyan(),
            required.dimmed(),
            page.title.bold()
        );
        println!("path: {}", page.path.display().to_string().dimmed());
        println!("{}", "---".dimmed());
        println!("{}", page.content.trim_end());
    }

    Ok(())
}

fn run_project_issues(project_root: &std::path::Path) -> Result<()> {
    let view = project_view::load_project_issues(project_root)?;

    println!(
        "{} Project issues: {}",
        "==>".blue().bold(),
        project_root.display().to_string().dimmed()
    );

    if view.issues.is_empty() {
        println!("No .project/issues/*.json files found.");
        return Ok(());
    }

    for entry in view.issues {
        let issue = entry.issue;
        let tags = if issue.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", issue.tags.join(","))
        };
        let blocker = if issue.is_blocker() { " blocked" } else { "" };
        println!(
            "{} {} [{}{}]{} {}",
            issue.short_id().cyan(),
            issue.title.bold(),
            issue.status.as_str(),
            blocker,
            tags.dimmed(),
            entry.path.display().to_string().dimmed()
        );
    }

    Ok(())
}

fn run_remote_doctor(project_root: &std::path::Path, skip_push: bool) -> Result<bool> {
    let report = remote::doctor_project(project_root, skip_push)?;

    println!(
        "{} Remote doctor: {}",
        "==>".blue().bold(),
        report.project_root.display().to_string().dimmed()
    );

    let contract_state = if report.project_contract.valid {
        remote::ProbeState::Pass
    } else {
        remote::ProbeState::Fail
    };
    println!(
        "{} .project contract: {} checks, {} warnings, {} errors",
        validation_prefix(contract_state),
        report.project_contract.checks_passed,
        report.project_contract.warnings,
        report.project_contract.errors
    );

    for message in &report.messages {
        println!("{} {}", validation_prefix(message.state), message.message);
    }

    for remote in &report.remotes {
        println!(
            "{} remote {} [{}] {}",
            "==>".blue().bold(),
            remote.endpoint.name.cyan(),
            remote.endpoint.kind.to_string().dimmed(),
            remote.endpoint.display_url().dimmed()
        );
        println!(
            "  {} fetch: {}",
            validation_prefix(remote.fetch.state),
            remote.fetch.message
        );
        println!(
            "  {} push: {}",
            validation_prefix(remote.push.state),
            remote.push.message
        );
    }

    if report.is_ok() {
        println!(
            "{} remote readiness valid ({} remote{})",
            "OK".green().bold(),
            report.remotes.len(),
            if report.remotes.len() == 1 { "" } else { "s" }
        );
    } else {
        println!(
            "{}",
            "FAIL remote readiness has blocking errors".red().bold()
        );
    }

    Ok(report.is_ok())
}

fn run_sober_action(project_root: &std::path::Path, action: SoberAction) -> Result<bool> {
    let repo = project_root.display().to_string();
    match action {
        SoberAction::Doctor { json, online } => {
            let mut args = vec!["doctor".to_string(), "--repo".to_string(), repo];
            if json {
                args.push("--json".to_string());
            }
            if online {
                args.push("--online".to_string());
            }
            sober::run(project_root, &args)
        }
        SoberAction::Preflight { base, json } => {
            let mut args = vec![
                "preflight".to_string(),
                "--repo".to_string(),
                repo,
                "--base".to_string(),
                base,
            ];
            if json {
                args.push("--json".to_string());
            }
            sober::run(project_root, &args)
        }
        SoberAction::Hygiene { profile, json } => {
            let mut args = vec![
                "hygiene".to_string(),
                "check".to_string(),
                "--repo".to_string(),
                repo,
                "--profile".to_string(),
                profile,
            ];
            if json {
                args.push("--json".to_string());
            }
            sober::run(project_root, &args)
        }
        SoberAction::ReviewPreview {
            base,
            provider,
            model,
            reviewer,
            objective,
            json,
        } => {
            let mut args = vec![
                "review".to_string(),
                "--repo".to_string(),
                repo,
                "--base".to_string(),
                base,
                "--reviewer".to_string(),
                reviewer,
                "--objective".to_string(),
                objective,
                "--prompt-preview".to_string(),
            ];
            if let Some(provider) = provider {
                args.extend(["--provider".to_string(), provider]);
            }
            if let Some(model) = model {
                args.extend(["--model".to_string(), model]);
            }
            if json {
                args.push("--json".to_string());
            }
            sober::run(project_root, &args)
        }
        SoberAction::Hooks { action } => match action {
            SoberHooksAction::Status { hook, json } => {
                run_sober_hook_action(project_root, "status", hook, json)
            }
            SoberHooksAction::Install { hook, json } => {
                run_sober_hook_action(project_root, "install", hook, json)
            }
        },
        SoberAction::Index { all, changed, json } => {
            let mut args = vec!["index".to_string(), "--repo".to_string(), repo];
            if changed {
                args.push("--changed".to_string());
            } else if all {
                args.push("--all".to_string());
            } else {
                args.push("--changed".to_string());
            }
            if json {
                args.push("--json".to_string());
            }
            sober::run(project_root, &args)
        }
        SoberAction::ForgeDoctor { json } => {
            let mut args = vec![
                "forge".to_string(),
                "doctor".to_string(),
                "--repo".to_string(),
                repo,
            ];
            if json {
                args.push("--json".to_string());
            }
            sober::run(project_root, &args)
        }
    }
}

fn run_sober_hook_action(
    project_root: &std::path::Path,
    method: &str,
    hook: Option<SoberHook>,
    json: bool,
) -> Result<bool> {
    if json && hook.is_none() {
        return run_sober_all_hooks_json(project_root, method);
    }

    let hooks = match hook {
        Some(hook) => vec![hook],
        None => vec![SoberHook::PreCommit, SoberHook::PrePush],
    };

    let commands = hooks
        .into_iter()
        .map(|hook| {
            let mut args = vec![
                "hooks".to_string(),
                method.to_string(),
                sober_hook_name(hook).to_string(),
            ];
            if json {
                args.push("--json".to_string());
            }
            args
        })
        .collect::<Vec<_>>();

    sober::run_many(project_root, &commands)
}

fn run_sober_all_hooks_json(project_root: &std::path::Path, method: &str) -> Result<bool> {
    let mut hooks = Vec::new();

    for hook in [SoberHook::PreCommit, SoberHook::PrePush] {
        let args = vec![
            "hooks".to_string(),
            method.to_string(),
            sober_hook_name(hook).to_string(),
            "--json".to_string(),
        ];
        let output = sober::output(project_root, &args)?;
        if !output.status.success() {
            use std::io::Write;

            let mut stderr = std::io::stderr();
            stderr.write_all(&output.stdout)?;
            stderr.write_all(&output.stderr)?;
            return Ok(false);
        }

        let value: serde_json::Value = serde_json::from_slice(&output.stdout)
            .with_context(|| format!("sober hooks {method} returned invalid JSON"))?;
        hooks.push(value);
    }

    println!("{}", serde_json::json!({ "ok": true, "hooks": hooks }));
    Ok(true)
}

fn sober_hook_name(hook: SoberHook) -> &'static str {
    match hook {
        SoberHook::PreCommit => "pre-commit",
        SoberHook::PrePush => "pre-push",
    }
}

fn validation_prefix(state: remote::ProbeState) -> colored::ColoredString {
    match state {
        remote::ProbeState::Pass => "OK".green().bold(),
        remote::ProbeState::Warn => "WARN".yellow().bold(),
        remote::ProbeState::Fail => "FAIL".red().bold(),
        remote::ProbeState::Skipped => "SKIP".dimmed(),
    }
}

fn print_validation_message(
    prefix: colored::ColoredString,
    message: &project_contract::ProjectValidationMessage,
) {
    if let Some(path) = &message.path {
        println!(
            "{} {}: {}",
            prefix,
            path.display().to_string().cyan(),
            message.message
        );
    } else {
        println!("{} {}", prefix, message.message);
    }
}

/// Handle progit:// URL scheme
#[cfg(feature = "forge-backend")]
fn handle_clone(endpoint: &str, repo: &str, dest: &std::path::Path) -> Result<()> {
    use colored::*;
    use progit::git::backend::{ForgedBackend, LocalGitBackend};
    use progit::git::clone::clone_repo;

    println!(
        "{} Cloning {} from {} into {}",
        "📦".cyan(),
        repo.bright_white(),
        endpoint.dimmed(),
        dest.display().to_string().dimmed()
    );

    // Build a tokio runtime on the fly — the rest of the CLI is sync.
    // The runtime is dropped after the clone completes.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| anyhow::anyhow!("tokio runtime: {e}"))?;

    runtime.block_on(async {
        let source = ForgedBackend::connect(endpoint.to_string())
            .await
            .map_err(|e| anyhow::anyhow!("connect to {endpoint}: {e}"))?;
        let local_dest = LocalGitBackend::new(dest)
            .map_err(|e| anyhow::anyhow!("init local backend at {}: {e}", dest.display()))?;

        let outcome = clone_repo(&source, repo, &local_dest, repo).await?;

        println!(
            "{} Cloned {} ref(s), {} accepted, {} rejected, {} pack bytes",
            "✓".green(),
            outcome.refs_total,
            outcome.refs_accepted,
            outcome.refs_rejected,
            outcome.pack_bytes
        );
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}

fn handle_deeplink(url: &str) -> Result<()> {
    use colored::*;

    let parts: Vec<&str> = url.split('/').collect();

    match parts[..] {
        ["install", plugin] => {
            let (name, version) = match plugin.split('@').collect::<Vec<_>>()[..] {
                [n, v] => (n, Some(v)),
                [n] => (n, None),
                _ => (plugin, None),
            };

            println!("{} Installing plugin '{}'", "📦".cyan(), name);
            if let Some(v) = version {
                println!("   Version: {}", v);
            }
            println!();
            println!("Run: {}", format!("prog plugin install {}", name).yellow());
            println!();
            println!(
                "Then verify: {}",
                format!("prog plugin verify {}", name).yellow()
            );
        }
        ["verify", plugin] => {
            println!("{} Verifying plugin '{}'", "🔍".cyan(), plugin);
            println!();
            println!("Run: {}", format!("prog plugin verify {}", plugin).yellow());
        }
        ["update", plugin] => {
            println!("{} Updating plugin '{}'", "🔄".cyan(), plugin);
            println!();
            println!("Run: {}", format!("prog plugin update {}", plugin).yellow());
        }
        ["uninstall", plugin] => {
            println!("{} Uninstalling plugin '{}'", "🗑️".cyan(), plugin);
            println!();
            println!("Run: {}", format!("prog plugin remove {}", plugin).yellow());
        }
        ["search", query] => {
            println!("{} Searching plugins for '{}'", "🔎".cyan(), query);
            println!();
            println!("Run: {}", format!("prog plugin search {}", query).yellow());
        }
        ["trust", keyid] => {
            println!("{} Trusting key '{}'", "🔑".cyan(), keyid);
            println!();
            println!("Run: {}", format!("prog trust add {}", keyid).yellow());
        }
        _ => {
            eprintln!("{} Unknown deeplink: progit://{}", "⚠️".yellow(), url);
            eprintln!("   Supported: install, verify, update, uninstall, search, trust");
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    // Initialize Logger — failure is non-fatal, continue without logging
    let _ = tui_logger::init_logger(log::LevelFilter::Trace);
    tui_logger::set_default_level(log::LevelFilter::Info);

    // 0. Check for deeplinks (progit://install/plugin)
    for arg in std::env::args() {
        if let Some(url) = arg.strip_prefix("progit://") {
            handle_deeplink(url)?;
            return Ok(());
        }
    }

    // 0b. Subcommands that should run BEFORE workspace detection.
    // `prog clone` writes a new local backend at the dest path; it does
    // NOT need an existing ProGit project at the current working directory.
    // We do a fast peek at argv[1] rather than parsing the full clap tree
    // here — full parse still happens later for normal commands.
    #[cfg(feature = "forge-backend")]
    {
        let argv: Vec<String> = std::env::args().collect();
        if argv.len() >= 2 && argv[1] == "clone" {
            let cli = Cli::parse();
            if let Some(Commands::Clone {
                endpoint,
                repo,
                dest,
            }) = cli.command
            {
                return handle_clone(&endpoint, &repo, &dest);
            }
        }
    }

    // `project` read commands must run before auto-initialization. Otherwise a
    // missing `.project/` would be created before read-only commands can report it.
    {
        let argv: Vec<String> = std::env::args().collect();
        if argv.get(1).map(String::as_str) == Some("project") {
            let cli = Cli::parse();
            if let Some(Commands::Project { action }) = cli.command {
                let project_root = workspace::find_project_root()?;
                if !run_project_action(&project_root, action)? {
                    std::process::exit(1);
                }
                return Ok(());
            }
        }
    }

    // `remote doctor` must also run before auto-initialization. It should
    // report missing project metadata; not silently create it first.
    {
        let argv: Vec<String> = std::env::args().collect();
        if argv.get(1).map(String::as_str) == Some("remote")
            && argv.get(2).map(String::as_str) == Some("doctor")
        {
            let cli = Cli::parse();
            if let Some(Commands::Remote {
                action: RemoteAction::Doctor { skip_push },
            }) = cli.command
            {
                let project_root = workspace::find_project_root()?;
                if !run_remote_doctor(&project_root, skip_push)? {
                    std::process::exit(1);
                }
                return Ok(());
            }
        }
    }

    // Sober is an external repository-governance helper and should inspect the
    // current repo as-is; do not auto-initialize ProGit metadata before it runs.
    {
        let argv: Vec<String> = std::env::args().collect();
        if argv.get(1).map(String::as_str) == Some("sober") {
            let cli = Cli::parse();
            if let Some(Commands::Sober { action }) = cli.command {
                let project_root = workspace::find_project_root()?;
                if !run_sober_action(&project_root, action)? {
                    std::process::exit(1);
                }
                return Ok(());
            }
        }
    }

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
                Some(MrAction::Review { action }) => match action {
                    ReviewAction::Push { mr_id } => {
                        if let Err(e) = run_review_push(&*provider, mr_id) {
                            eprintln!("{} {}", "❌".red(), e);
                            std::process::exit(1);
                        }
                    }
                },
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
        Some(Commands::Trust { action }) => {
            handle_trust_command(action)?;
        }
        Some(Commands::Project { action }) => {
            if !run_project_action(&project_root, action)? {
                std::process::exit(1);
            }
        }
        Some(Commands::Remote { action }) => match action {
            RemoteAction::Doctor { skip_push } => {
                if !run_remote_doctor(&project_root, skip_push)? {
                    std::process::exit(1);
                }
            }
        },
        Some(Commands::Sober { action }) => {
            if !run_sober_action(&project_root, action)? {
                std::process::exit(1);
            }
        }
        Some(Commands::Hooks { action }) => {
            cli::handle_hooks_command(action, &project_root)?;
        }
        Some(Commands::RebaseEditor { path }) => {
            crate::rebase::run(&path)?;
        }
        #[cfg(feature = "forge-backend")]
        Some(Commands::Clone {
            endpoint,
            repo,
            dest,
        }) => {
            handle_clone(&endpoint, &repo, &dest)?;
        }
        None => {
            // No command - run TUI
            runner::start_tui()?;
        }
    }

    Ok(())
}
