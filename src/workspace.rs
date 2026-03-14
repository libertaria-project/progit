//! Workspace - Project root detection and initialization

use anyhow::Result;
use colored::*;
use std::path::{Path, PathBuf};

use crate::storage;

/// Find the project root (directory containing .git or .project)
pub(crate) fn find_project_root() -> Result<PathBuf> {
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
pub(crate) fn initialize_workspace(root: &Path) -> Result<()> {
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

pub(crate) fn auto_configure(sync_config: &mut storage::config::SyncConfig, cwd: &std::path::Path) {
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
                        // SAFETY: both indices guarded by len >= 2 check above
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

/// Save all issues (for drag-drop operations)
pub(crate) fn save_all_issues(
    issues: &[crate::issue::Issue],
    kdl_dir: &std::path::Path,
    cache_path: &std::path::Path,
) -> Result<()> {
    use crate::storage::save_issue;
    for issue in issues {
        save_issue(issue, kdl_dir, cache_path)?;
    }
    Ok(())
}
