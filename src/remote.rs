//! Remote adapter and doctor checks for repository-centric ProGit remotes.

use crate::project_contract;
use anyhow::{Context, Result};
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Normalized remote provider kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteKind {
    /// Local path or file URL remote.
    Local,
    /// GitHub-hosted Git remote.
    GitHub,
    /// GitLab-hosted Git remote.
    GitLab,
    /// Forgejo/Gitea-compatible Git remote.
    Forgejo,
    /// Unknown plain Git remote.
    PlainGit,
}

impl fmt::Display for RemoteKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local => f.write_str("local"),
            Self::GitHub => f.write_str("github"),
            Self::GitLab => f.write_str("gitlab"),
            Self::Forgejo => f.write_str("forgejo"),
            Self::PlainGit => f.write_str("git"),
        }
    }
}

impl RemoteKind {
    /// Classify a Git remote URL without requiring provider API access.
    pub fn from_url(url: &str) -> Self {
        let lower = url.to_ascii_lowercase();
        if is_local_remote_url(&lower) {
            return Self::Local;
        }

        let host = remote_host(&lower);
        if host.contains("github.com") {
            Self::GitHub
        } else if host.contains("gitlab.") || host == "gitlab.com" {
            Self::GitLab
        } else if host.contains("forgejo")
            || host.contains("gitea")
            || host == "codeberg.org"
            || host == "git.sovereign-society.org"
        {
            Self::Forgejo
        } else {
            Self::PlainGit
        }
    }
}

/// Configured remote endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteEndpoint {
    /// Git remote name, for example `origin`.
    pub name: String,
    /// Raw remote URL from Git config.
    pub url: String,
    /// Best-effort provider classification.
    pub kind: RemoteKind,
}

impl RemoteEndpoint {
    fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        let url = url.into();
        Self {
            name: name.into(),
            kind: RemoteKind::from_url(&url),
            url,
        }
    }

    /// URL suitable for CLI display. Credentials are redacted.
    pub fn display_url(&self) -> String {
        redact_remote_url(&self.url)
    }
}

/// Probe result state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeState {
    /// Check passed.
    Pass,
    /// Check found a non-fatal issue.
    Warn,
    /// Check failed and should make doctor fail.
    Fail,
    /// Check was intentionally skipped.
    Skipped,
}

impl ProbeState {
    /// Returns true when the probe is a hard failure.
    pub fn is_failure(self) -> bool {
        self == Self::Fail
    }
}

/// Individual doctor probe result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteProbe {
    /// Probe status.
    pub state: ProbeState,
    /// Short human-readable message.
    pub message: String,
}

impl RemoteProbe {
    fn pass(message: impl Into<String>) -> Self {
        Self {
            state: ProbeState::Pass,
            message: message.into(),
        }
    }

    fn warn(message: impl Into<String>) -> Self {
        Self {
            state: ProbeState::Warn,
            message: message.into(),
        }
    }

    fn fail(message: impl Into<String>) -> Self {
        Self {
            state: ProbeState::Fail,
            message: message.into(),
        }
    }

    fn skipped(message: impl Into<String>) -> Self {
        Self {
            state: ProbeState::Skipped,
            message: message.into(),
        }
    }
}

/// Contract validation summary used by `remote doctor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectContractProbe {
    /// True when the repository-owned project contract is valid.
    pub valid: bool,
    /// Number of checks passed by the project contract validator.
    pub checks_passed: usize,
    /// Number of warnings emitted by the project contract validator.
    pub warnings: usize,
    /// Number of errors emitted by the project contract validator.
    pub errors: usize,
}

impl ProjectContractProbe {
    fn state(&self) -> ProbeState {
        if self.valid {
            ProbeState::Pass
        } else {
            ProbeState::Fail
        }
    }
}

/// Full check result for one remote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteCheck {
    /// Remote endpoint.
    pub endpoint: RemoteEndpoint,
    /// Read/fetch reachability probe.
    pub fetch: RemoteProbe,
    /// Dry-run push probe.
    pub push: RemoteProbe,
}

/// Full `remote doctor` report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteDoctorReport {
    /// Repository root used for the checks.
    pub project_root: PathBuf,
    /// Project contract status.
    pub project_contract: ProjectContractProbe,
    /// Non-remote-specific messages.
    pub messages: Vec<RemoteProbe>,
    /// Per-remote checks.
    pub remotes: Vec<RemoteCheck>,
}

impl RemoteDoctorReport {
    /// Returns true if any hard failure was found.
    pub fn has_failures(&self) -> bool {
        self.project_contract.state().is_failure()
            || self.messages.iter().any(|probe| probe.state.is_failure())
            || self
                .remotes
                .iter()
                .any(|remote| remote.fetch.state.is_failure() || remote.push.state.is_failure())
    }

    /// Returns true if all hard checks passed.
    pub fn is_ok(&self) -> bool {
        !self.has_failures()
    }
}

/// Adapter interface for normalized remote health checks.
pub trait RemoteAdapter {
    /// List configured remotes.
    fn list_remotes(&self) -> Result<Vec<RemoteEndpoint>>;

    /// Check read/fetch reachability.
    fn check_fetch(&self, remote: &RemoteEndpoint) -> RemoteProbe;

    /// Check push capability without mutating the remote.
    fn check_push_dry_run(&self, remote: &RemoteEndpoint) -> RemoteProbe;
}

/// Git-config-backed remote adapter.
pub struct GitRemoteAdapter {
    root: PathBuf,
}

impl GitRemoteAdapter {
    /// Create an adapter rooted at a local Git repository.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn run_git_probe(&self, args: &[&str], success: &str, failure_prefix: &str) -> RemoteProbe {
        let output = Command::new("git")
            .current_dir(&self.root)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GCM_INTERACTIVE", "never")
            .args(args)
            .output();

        match output {
            Ok(output) if output.status.success() => RemoteProbe::pass(success),
            Ok(output) => RemoteProbe::fail(format!(
                "{failure_prefix}: {}",
                summarize_git_output(&output.stderr, &output.stdout)
            )),
            Err(err) => RemoteProbe::fail(format!("{failure_prefix}: failed to run git: {err}")),
        }
    }
}

impl RemoteAdapter for GitRemoteAdapter {
    fn list_remotes(&self) -> Result<Vec<RemoteEndpoint>> {
        let repo = git2::Repository::discover(&self.root)
            .with_context(|| format!("Not a Git repository: {}", self.root.display()))?;
        let remotes = repo.remotes().context("Failed to read Git remotes")?;

        let mut endpoints = Vec::new();
        for name in remotes.iter().flatten() {
            let remote = repo
                .find_remote(name)
                .with_context(|| format!("Failed to read remote `{name}`"))?;
            if let Some(url) = remote.url() {
                endpoints.push(RemoteEndpoint::new(name, url));
            }
        }
        endpoints.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(endpoints)
    }

    fn check_fetch(&self, remote: &RemoteEndpoint) -> RemoteProbe {
        self.run_git_probe(
            &["ls-remote", "--heads", &remote.name],
            "read/fetch probe passed",
            "read/fetch probe failed",
        )
    }

    fn check_push_dry_run(&self, remote: &RemoteEndpoint) -> RemoteProbe {
        self.run_git_probe(
            &["push", "--dry-run", "--porcelain", &remote.name, "HEAD"],
            "dry-run push probe passed",
            "dry-run push probe failed",
        )
    }
}

/// Run remote doctor against the project root.
pub fn doctor_project(root: &Path, skip_push: bool) -> Result<RemoteDoctorReport> {
    let project_report = project_contract::validate_project(root)?;
    let project_contract = ProjectContractProbe {
        valid: project_report.is_valid(),
        checks_passed: project_report.checks_passed,
        warnings: project_report.warnings.len(),
        errors: project_report.errors.len(),
    };

    let adapter = GitRemoteAdapter::new(root);
    let remotes = adapter.list_remotes()?;
    let mut messages = Vec::new();
    if remotes.is_empty() {
        messages.push(RemoteProbe::warn("no Git remotes configured"));
    }

    let remote_checks = remotes
        .into_iter()
        .map(|endpoint| {
            let fetch = adapter.check_fetch(&endpoint);
            let push = if skip_push {
                RemoteProbe::skipped("dry-run push probe skipped")
            } else {
                adapter.check_push_dry_run(&endpoint)
            };
            RemoteCheck {
                endpoint,
                fetch,
                push,
            }
        })
        .collect();

    Ok(RemoteDoctorReport {
        project_root: root.to_path_buf(),
        project_contract,
        messages,
        remotes: remote_checks,
    })
}

fn is_local_remote_url(url: &str) -> bool {
    url.starts_with("file://")
        || url.starts_with('/')
        || url.starts_with("./")
        || url.starts_with("../")
        || url.ends_with(".git") && !url.contains("://") && !url.contains('@') && !url.contains(':')
}

fn remote_host(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("git@") {
        return rest
            .split(':')
            .next()
            .unwrap_or_default()
            .trim_matches('/')
            .to_string();
    }

    if let Some((_, rest)) = url.split_once("://") {
        return rest
            .split('/')
            .next()
            .unwrap_or_default()
            .split('@')
            .next_back()
            .unwrap_or_default()
            .to_string();
    }

    String::new()
}

fn redact_remote_url(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_string();
    };
    let Some((credentials, host_and_path)) = rest.split_once('@') else {
        return url.to_string();
    };
    if credentials.is_empty() {
        url.to_string()
    } else {
        format!("{scheme}://<redacted>@{host_and_path}")
    }
}

fn summarize_git_output(stderr: &[u8], stdout: &[u8]) -> String {
    let raw = if stderr.is_empty() { stdout } else { stderr };
    let text = String::from_utf8_lossy(raw);
    let first_line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("git returned no diagnostic output");
    first_line.chars().take(240).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn classifies_common_remote_urls() {
        assert_eq!(
            RemoteKind::from_url("https://github.com/owner/repo.git"),
            RemoteKind::GitHub
        );
        assert_eq!(
            RemoteKind::from_url("git@gitlab.com:group/repo.git"),
            RemoteKind::GitLab
        );
        assert_eq!(
            RemoteKind::from_url("https://git.sovereign-society.org/ProGit/progit.git"),
            RemoteKind::Forgejo
        );
        assert_eq!(RemoteKind::from_url("../remote.git"), RemoteKind::Local);
        assert_eq!(
            RemoteKind::from_url("ssh://git@example.org/team/repo.git"),
            RemoteKind::PlainGit
        );
    }

    #[test]
    fn redacts_http_credentials() {
        assert_eq!(
            redact_remote_url("https://user:token@example.org/team/repo.git"),
            "https://<redacted>@example.org/team/repo.git"
        );
        assert_eq!(
            redact_remote_url("https://example.org/team/repo.git"),
            "https://example.org/team/repo.git"
        );
    }

    #[test]
    fn warns_when_no_remotes_are_configured() {
        let dir = tempdir().unwrap();
        git(dir.path(), &["init"]);

        let adapter = GitRemoteAdapter::new(dir.path());
        let remotes = adapter.list_remotes().unwrap();

        assert!(remotes.is_empty());
    }

    #[test]
    fn local_bare_remote_passes_fetch_and_push_dry_run() {
        let dir = tempdir().unwrap();
        let work = dir.path().join("work");
        let remote = dir.path().join("remote.git");
        fs::create_dir_all(&work).unwrap();

        git(&work, &["init"]);
        git(&work, &["checkout", "-b", "main"]);
        git(&work, &["config", "user.email", "test@example.org"]);
        git(&work, &["config", "user.name", "ProGit Test"]);
        git(&work, &["config", "commit.gpgsign", "false"]);
        fs::write(work.join("README.md"), "# test\n").unwrap();
        git(&work, &["add", "README.md"]);
        git(&work, &["commit", "-m", "init"]);

        git_path(dir.path(), &["init", "--bare"], &remote);
        git_path(&work, &["remote", "add", "origin"], &remote);
        git(&work, &["push", "-u", "origin", "main"]);

        let adapter = GitRemoteAdapter::new(&work);
        let remotes = adapter.list_remotes().unwrap();
        assert_eq!(remotes.len(), 1);
        assert_eq!(remotes[0].kind, RemoteKind::Local);

        let fetch = adapter.check_fetch(&remotes[0]);
        let push = adapter.check_push_dry_run(&remotes[0]);

        assert_eq!(fetch.state, ProbeState::Pass, "{fetch:?}");
        assert_eq!(push.state, ProbeState::Pass, "{push:?}");
    }

    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .current_dir(dir)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .arg("-c")
            .arg("init.defaultBranch=main")
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {:?} failed", args);
    }

    fn git_path(dir: &Path, args: &[&str], path: &Path) {
        let status = Command::new("git")
            .current_dir(dir)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .arg("-c")
            .arg("init.defaultBranch=main")
            .args(args)
            .arg(path)
            .status()
            .unwrap();
        assert!(status.success(), "git {:?} {} failed", args, path.display());
    }
}
