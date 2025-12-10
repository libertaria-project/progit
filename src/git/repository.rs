//! Git Repository - Repository detection and info
//!
//! Detects git repository and provides branch/remote information.

use anyhow::{Context, Result};
use git2::{BranchType, Repository as Git2Repo, StatusOptions};
use std::path::Path;

/// Git repository information
#[derive(Debug, Clone)]
pub struct RepoInfo {
    /// Local path to repository root
    pub path: String,

    /// Current branch name
    pub branch: String,

    /// Remote name (e.g., "origin")
    pub remote_name: Option<String>,

    /// Remote URL
    pub remote_url: Option<String>,

    /// Number of commits ahead of remote
    pub ahead: usize,

    /// Number of commits behind remote
    pub behind: usize,

    /// Number of modified files
    pub modified: usize,

    /// Number of untracked files
    pub untracked: usize,

    /// All available locally known branches
    pub branches: Vec<String>,

    /// All available remotes
    pub remotes: Vec<RemoteInfo>,
}

/// Remote information
#[derive(Debug, Clone)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
}

impl Default for RepoInfo {
    fn default() -> Self {
        Self {
            path: String::new(),
            branch: "main".to_string(),
            remote_name: None,
            remote_url: None,
            ahead: 0,
            behind: 0,
            modified: 0,
            untracked: 0,
            branches: Vec::new(),
            remotes: Vec::new(),
        }
    }
}

/// Detect git repository from a path
pub fn detect_repo(start_path: &Path) -> Result<Option<RepoInfo>> {
    // Try to open repository (searches upward)
    let repo = match Git2Repo::discover(start_path) {
        Ok(r) => r,
        Err(e) => {
             // Check for SHA256 error
             if e.message().contains("sha256") {
                 let mut info = RepoInfo::default();
                 info.branch = "UNKNOWN (SHA-256)".to_string();
                 info.remote_url = Some("Not supported by libgit2 yet".to_string());
                 // Try to at least show the path
                 info.path = start_path.to_string_lossy().to_string();
                 return Ok(Some(info));
             }
             return Ok(None);
        },
    };

    let mut info = RepoInfo::default();

    // Get repository path
    if let Some(workdir) = repo.workdir() {
        info.path = workdir.to_string_lossy().to_string();
    }

    // Get current branch
    if let Ok(head) = repo.head() {
        if let Some(name) = head.shorthand() {
            info.branch = name.to_string();
        }
    }

    // Get local branches
    if let Ok(branches) = repo.branches(Some(BranchType::Local)) {
        for branch in branches.flatten() {
            if let Ok(Some(name)) = branch.0.name() {
                info.branches.push(name.to_string());
            }
        }
        info.branches.sort();
    }

    // Get remotes
    if let Ok(remotes) = repo.remotes() {
        for remote_name in remotes.iter().flatten() {
            if let Ok(remote) = repo.find_remote(remote_name) {
                if let Some(url) = remote.url() {
                    info.remotes.push(RemoteInfo {
                        name: remote_name.to_string(),
                        url: url.to_string(),
                    });

                    // Set first remote as default
                    if info.remote_name.is_none() {
                        info.remote_name = Some(remote_name.to_string());
                        info.remote_url = Some(url.to_string());
                    }
                }
            }
        }
    }

    // Get ahead/behind for current branch
    if let (Some(remote_name), Ok(head)) = (&info.remote_name, repo.head()) {
        let upstream_name = format!("{}/{}", remote_name, info.branch);
        if let Ok(upstream) = repo.find_branch(&upstream_name, BranchType::Remote) {
            if let (Ok(local_oid), Ok(upstream_ref)) = (head.target().context("No HEAD target"), upstream.get().target().context("No upstream target")) {
                if let Ok((ahead, behind)) = repo.graph_ahead_behind(local_oid, upstream_ref) {
                    info.ahead = ahead;
                    info.behind = behind;
                }
            }
        }
    }

    // Get file status
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    opts.recurse_untracked_dirs(false);

    if let Ok(statuses) = repo.statuses(Some(&mut opts)) {
        for entry in statuses.iter() {
            let status = entry.status();
            if status.is_wt_new() {
                info.untracked += 1;
            } else if status.is_wt_modified()
                || status.is_wt_deleted()
                || status.is_index_modified()
                || status.is_index_new()
                || status.is_index_deleted()
            {
                info.modified += 1;
            }
        }
    }

    Ok(Some(info))
}

/// Refresh repository info
pub fn refresh_repo(path: &Path) -> Result<Option<RepoInfo>> {
    detect_repo(path)
}

/// Format remote URL for display (shorten if needed)
pub fn format_remote_url(url: &str) -> String {
    // Convert git@github.com:user/repo.git to github.com/user/repo
    if url.starts_with("git@") {
        let trimmed = url.trim_start_matches("git@").replace(':', "/");
        trimmed.trim_end_matches(".git").to_string()
    } else {
        url.trim_end_matches(".git").to_string()
    }
}

/// Switch branch
pub fn switch_branch(path: &Path, branch: &str) -> Result<()> {
    let repo = Git2Repo::open(path)?;
    let (object, reference) = repo.revparse_ext(branch)?;
    repo.checkout_tree(&object, None)?;
    repo.set_head(reference.unwrap().name().unwrap())?;
    Ok(())
}

/// Create new branch
pub fn create_branch(path: &Path, name: &str) -> Result<()> {
    let repo = Git2Repo::open(path)?;
    let head = repo.head()?.peel_to_commit()?;
    repo.branch(name, &head, false)?;
    // Checkout immediately
    switch_branch(path, name)
}

/// Delete a branch (cannot delete current branch)
pub fn delete_branch(path: &Path, name: &str) -> Result<()> {
    let repo = Git2Repo::open(path)?;
    
    // Check if it's the current branch
    if let Ok(head) = repo.head() {
        if let Some(current) = head.shorthand() {
            if current == name {
                anyhow::bail!("Cannot delete current branch");
            }
        }
    }
    
    let mut branch = repo.find_branch(name, BranchType::Local)?;
    branch.delete()?;
    Ok(())
}

/// Get origin repository URL
pub fn get_origin_url(path: &Path) -> Result<Option<String>> {
    let repo = Git2Repo::open(path)?;
    Ok(repo.find_remote("origin").ok().and_then(|remote| remote.url().map(|s| s.to_string())))
}

/// Parse git URL into (host, owner, repo)
pub fn parse_git_url(url: &str) -> Option<(String, String, String)> {
    // Supports:
    // https://github.com/user/repo.git
    // git@github.com:user/repo.git
    
    let trimmed = url.trim_end_matches(".git");
    
    if trimmed.starts_with("http") {
        if let Some(pos) = trimmed.find("://") {
            let parts: Vec<&str> = trimmed[pos+3..].split('/').collect();
            if parts.len() >= 3 {
                return Some(("https://".to_string() + parts[0], parts[1].to_string(), parts[2].to_string()));
            }
        }
    } else if trimmed.starts_with("git@") {
        let trimmed = trimmed.trim_start_matches("git@");
        if let Some(colon) = trimmed.find(':') {
            let host = &trimmed[..colon];
            let path = &trimmed[colon+1..];
            let parts: Vec<&str> = path.split('/').collect();
            if parts.len() >= 2 {
                return Some(("https://".to_string() + host, parts[0].to_string(), parts[1].to_string()));
            }
        }
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_remote_url_ssh() {
        let url = "git@github.com:markus/projectstui.git";
        assert_eq!(format_remote_url(url), "github.com/markus/projectstui");
    }

    #[test]
    fn test_format_remote_url_https() {
        let url = "https://github.com/markus/projectstui.git";
        assert_eq!(format_remote_url(url), "github.com/markus/projectstui");
    }

    #[test]
    fn test_detect_repo_nonexistent() {
        let result = detect_repo(Path::new("/nonexistent/path"));
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
