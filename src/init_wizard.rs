//! Init Wizard - Guided setup for new ProGit projects
//!
//! Seeds a demo project with sample issues, a kanban board, and a mock MR.
//! Interactive by default; `--yes` flag enables non-interactive CI use.

use anyhow::Result;
use colored::*;
use std::io::{self, Write};
use std::path::Path;

use crate::issue::{Effort, Issue, Status};
use crate::mr::MergeRequest;
use crate::storage::engine::StorageEngine;
use crate::workspace;

/// Wizard options (populated from CLI flags)
pub struct InitOptions {
    /// Seed demo data without asking
    pub demo: bool,
    /// Install git hooks without asking
    pub hooks: bool,
    /// Skip all prompts (non-interactive / CI mode)
    pub yes: bool,
}

/// Run the interactive (or flag-driven) init wizard.
pub fn run_wizard(project_root: &Path, options: InitOptions) -> Result<()> {
    println!("{}  {}", "⚔️".cyan(), "ProGit Init Wizard".bold().cyan());
    println!("{}", "─".repeat(52));

    // 1. Prerequisites
    let has_git = crate::git::detect_repo(project_root)?.is_some();
    if !has_git {
        println!("{} No git repository found.", "❌".red());
        println!("   ProGit requires a git repository. Run: {}", "git init".cyan());
        return Ok(());
    }

    // 2. Base workspace setup
    let project_dir = project_root.join(crate::storage::paths::PROJECT_DIR);
    let already_initialized = project_dir.exists();

    if already_initialized {
        if !options.yes {
            let cont = ask_bool(
                &format!(
                    "{} already exists. Add demo data anyway?",
                    ".project/".dimmed()
                ),
                true,
            )?;
            if !cont {
                println!("{} Aborted.", "🚫".yellow());
                return Ok(());
            }
        }
    } else {
        println!("{} Initializing workspace...", "🔧".blue());
        workspace::initialize_workspace(project_root)?;
    }

    // 3. Demo data
    let with_demo = if options.yes {
        options.demo
    } else {
        ask_bool("Seed demo data (sample issues + kanban board + mock MR)?", true)?
    };

    if with_demo {
        println!("{} Planting demo garden...", "🌱".green());
        seed_demo_data(project_root)?;
    }

    // 4. Git hooks
    let with_hooks = if options.yes {
        options.hooks
    } else {
        ask_bool(
            "Install git hooks (auto-update issues from commit messages)?",
            true,
        )?
    };

    if with_hooks {
        println!("{} Installing git hooks...", "🔧".blue());
        match crate::hooks::install_hooks(project_root) {
            Ok(installed) => {
                for hook in &installed {
                    println!("   {} {}", "✓".green(), hook.filename());
                }
            }
            Err(e) => {
                println!("{} Failed to install hooks: {}", "⚠️".yellow(), e);
            }
        }
    }

    // 5. Victory lap
    println!();
    println!("{}", "🎉  ProGit is ready!".bold().green());
    println!(
        "   Run {} to open the terminal cockpit.",
        "prog".cyan().bold()
    );
    if with_demo {
        println!(
            "   Your demo has {} issues across all kanban columns and {} mock MR.",
            "5".green(),
            "1".green()
        );
    }
    println!(
        "   Customize settings in {}.",
        ".project/config.kdl".dimmed()
    );
    println!(
        "   {} Canonical home: {}",
        "🌐".blue(),
        "https://progit.dev".cyan().underline()
    );

    Ok(())
}

/// Prompt the user for a yes/no answer.
fn ask_bool(prompt: &str, default: bool) -> Result<bool> {
    let hint = if default { "[Y/n]" } else { "[y/N]" };
    print!("{} {} ", prompt.yellow(), hint.dimmed());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let trimmed = input.trim().to_lowercase();
    if trimmed.is_empty() {
        Ok(default)
    } else {
        Ok(trimmed == "y" || trimmed == "yes")
    }
}

/// Plant sample issues and a mock MR so the kanban board is alive on first run.
fn seed_demo_data(project_root: &Path) -> Result<()> {
    let mut engine = StorageEngine::new(project_root);
    engine.load()?;

    // ── Sample Issues ──────────────────────────────────────────────
    // Designed to populate all three kanban columns with realistic variety.

    let issues = vec![
        // In Progress column
        Issue::new("Set up CI/CD pipeline")
            .with_description(
                "Configure automated builds, tests, and signed releases.\n\
                 Target: Forgejo Actions → minisign → GitHub mirror.",
            )
            .with_status(Status::InProgress)
            .with_effort(Effort::Medium)
            .with_tags(vec!["devops".to_string(), "blocker".to_string()])
            .with_assignee("alex"),
        Issue::new("Migrate database schema v1 → v2")
            .with_description(
                "Zero-downtime migration with rollback strategy.\n\
                 Includes data validation scripts and dry-run mode.",
            )
            .with_status(Status::InProgress)
            .with_effort(Effort::Epic)
            .with_tags(vec!["backend".to_string(), "blocker".to_string()])
            .with_assignee("sam"),
        // Done column
        Issue::new("Design onboarding wizard")
            .with_description(
                "First-run experience: guided init, demo data seed,\n\
                 empty-state teaching. Success = 60s to first value.",
            )
            .with_status(Status::Done)
            .with_effort(Effort::Small)
            .with_tags(vec!["ux".to_string(), "onboarding".to_string()])
            .with_assignee("alex"),
        // Backlog column
        Issue::new("Fix login redirect after OAuth")
            .with_description(
                "Users land on /login instead of /dashboard after GitLab SSO.\n\
                 Repro: Chrome incognito, third-party cookies blocked.",
            )
            .with_status(Status::Backlog)
            .with_effort(Effort::Small)
            .with_tags(vec!["bug".to_string(), "auth".to_string()]),
        Issue::new("Write API documentation")
            .with_description(
                "Document all REST endpoints with curl examples\n\
                 and OpenAPI schema. Publish to docs site.",
            )
            .with_status(Status::Backlog)
            .with_effort(Effort::Large)
            .with_tags(vec!["docs".to_string()]),
    ];

    for issue in issues {
        engine.upsert(issue)?;
    }

    // ── Mock Merge Request ─────────────────────────────────────────
    let mr = MergeRequest::new("feature/dark-mode", "main", "feat: add dark mode support")
        .with_description(
            "Implements a toggleable dark theme using CSS custom properties.\n\
             Respects `prefers-color-scheme` and persists user choice.",
        )
        .with_assignee("alex")
        .with_labels(vec!["frontend".to_string(), "ui".to_string()]);

    engine.upsert_mr(mr)?;

    println!(
        "   {} issues created (Backlog: 2, In Progress: 2, Done: 1)",
        engine.issues().len().to_string().green().bold()
    );
    println!(
        "   {} merge request created (Open)",
        engine.mrs().len().to_string().green().bold()
    );

    Ok(())
}
