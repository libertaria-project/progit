// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2026 Markus Maiwald

//! Narrow host bridge to the Sober repository governance helper.
//!
//! ProGit intentionally does not give Lua plugins arbitrary process execution.
//! This module is the first safe bridge: a small, reviewable allowlist of Sober
//! commands that ProGit can expose through CLI/TUI/plugin surfaces.

use anyhow::{anyhow, Context, Result};
use std::path::Path;
use std::process::{Command, Output};

const SOBER_BIN: &str = "sober";

/// Run one Sober command in `repo_root`, streaming output to the terminal.
pub fn run(repo_root: &Path, args: &[String]) -> Result<bool> {
    let status = Command::new(SOBER_BIN)
        .args(args)
        .current_dir(repo_root)
        .status()
        .with_context(|| {
            format!(
                "failed to run `{}`. Install Sober or put it in PATH.",
                SOBER_BIN
            )
        })?;

    Ok(status.success())
}

/// Run multiple Sober commands, stopping on the first failure.
pub fn run_many(repo_root: &Path, commands: &[Vec<String>]) -> Result<bool> {
    for args in commands {
        if !run(repo_root, args)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Run one Sober command and capture its output.
pub fn output(repo_root: &Path, args: &[String]) -> Result<Output> {
    Command::new(SOBER_BIN)
        .args(args)
        .current_dir(repo_root)
        .output()
        .with_context(|| {
            format!(
                "failed to run `{}`. Install Sober or put it in PATH.",
                SOBER_BIN
            )
        })
}

/// Convert a TUI `:sober ...` command into a safe `prog sober ...` process.
pub fn tui_command_args(parts: &[&str]) -> Result<Vec<String>> {
    let current_exe = std::env::current_exe().unwrap_or_else(|_| "prog".into());
    let mut args = vec![current_exe.display().to_string(), "sober".to_string()];
    args.extend(validate_tui_args(parts)?);
    Ok(args)
}

fn validate_tui_args(parts: &[&str]) -> Result<Vec<String>> {
    if parts.is_empty() {
        return Err(anyhow!(
            "Usage: :sober <doctor|preflight|hygiene|review-preview|hooks|index|forge-doctor>"
        ));
    }

    match parts[0] {
        "doctor" | "preflight" | "hygiene" | "review-preview" | "index" | "forge-doctor" => {
            Ok(parts.iter().map(|part| (*part).to_string()).collect())
        }
        "hooks" => validate_hooks_args(parts),
        other => Err(anyhow!("Unsupported Sober command: {other}")),
    }
}

fn validate_hooks_args(parts: &[&str]) -> Result<Vec<String>> {
    match parts {
        ["hooks", "status" | "install"] => {
            Ok(parts.iter().map(|part| (*part).to_string()).collect())
        }
        ["hooks", "status" | "install", "--json"] => {
            Ok(parts.iter().map(|part| (*part).to_string()).collect())
        }
        ["hooks", "status" | "install", "pre-commit" | "pre-push"] => {
            Ok(parts.iter().map(|part| (*part).to_string()).collect())
        }
        ["hooks", "status" | "install", "pre-commit" | "pre-push", "--json"] => {
            Ok(parts.iter().map(|part| (*part).to_string()).collect())
        }
        _ => Err(anyhow!(
            "Usage: :sober hooks <status|install [pre-commit|pre-push]>"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tui_allows_review_preview() {
        let args = validate_tui_args(&["review-preview", "--provider", "kimi-coding"]).unwrap();

        assert_eq!(args, ["review-preview", "--provider", "kimi-coding"]);
    }

    #[test]
    fn tui_rejects_arbitrary_subcommands() {
        let err = validate_tui_args(&["exec", "rm"]).unwrap_err().to_string();

        assert!(err.contains("Unsupported Sober command"));
    }

    #[test]
    fn tui_rejects_unknown_hook_names() {
        let err = validate_tui_args(&["hooks", "install", "post-checkout"])
            .unwrap_err()
            .to_string();

        assert!(err.contains("Usage: :sober hooks"));
    }

    #[test]
    fn tui_allows_hook_status_json() {
        let args = validate_tui_args(&["hooks", "status", "pre-commit", "--json"]).unwrap();

        assert_eq!(args, ["hooks", "status", "pre-commit", "--json"]);
    }
}
