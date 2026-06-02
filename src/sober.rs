// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2026 Markus Maiwald

//! Narrow host bridge to the Sober repository governance helper.
//!
//! ProGit intentionally does not give Lua plugins arbitrary process execution.
//! This module is the first safe bridge: a small, reviewable allowlist of Sober
//! commands that ProGit can expose through CLI/TUI/plugin surfaces.

use anyhow::{anyhow, Context, Result};
use progit_plugin_sdk::lua::{SoberHost, SoberInvocation, SoberInvocationResult};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
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

/// Create the sandboxed plugin host capability for Sober.
pub fn host_capability(repo_root: PathBuf) -> SoberHost {
    SoberHost::new(move |invocation| invoke_for_plugin(&repo_root, invocation))
}

fn invoke_for_plugin(repo_root: &Path, invocation: SoberInvocation) -> SoberInvocationResult {
    match plugin_command_plan(repo_root, &invocation) {
        Ok(PluginCommandPlan::Single(args)) => match run_json(repo_root, &args) {
            Ok(data) => SoberInvocationResult {
                ok: true,
                data,
                error: None,
            },
            Err(error) => failed(error),
        },
        Ok(PluginCommandPlan::Aggregate { key, commands }) => {
            let mut items = Vec::new();
            for args in commands {
                match run_json(repo_root, &args) {
                    Ok(data) => items.push(data),
                    Err(error) => return failed(error),
                }
            }
            SoberInvocationResult {
                ok: true,
                data: json!({ key: items }),
                error: None,
            }
        }
        Err(error) => failed(error.to_string()),
    }
}

#[derive(Debug)]
enum PluginCommandPlan {
    Single(Vec<String>),
    Aggregate {
        key: &'static str,
        commands: Vec<Vec<String>>,
    },
}

fn plugin_command_plan(repo_root: &Path, invocation: &SoberInvocation) -> Result<PluginCommandPlan> {
    let repo = repo_root.display().to_string();
    let options = &invocation.options;

    match invocation.action.as_str() {
        "doctor" => {
            let mut args = vec!["doctor".to_string(), "--repo".to_string(), repo];
            if option_bool(options, "online", false) {
                args.push("--online".to_string());
            }
            args.push("--json".to_string());
            Ok(PluginCommandPlan::Single(args))
        }
        "preflight" => Ok(PluginCommandPlan::Single(vec![
            "preflight".to_string(),
            "--repo".to_string(),
            repo,
            "--base".to_string(),
            option_str(options, "base", "HEAD")?,
            "--json".to_string(),
        ])),
        "hygiene" => Ok(PluginCommandPlan::Single(vec![
            "hygiene".to_string(),
            "check".to_string(),
            "--repo".to_string(),
            repo,
            "--profile".to_string(),
            option_str(options, "profile", "standard")?,
            "--json".to_string(),
        ])),
        "review-preview" => {
            let mut args = vec![
                "review".to_string(),
                "--repo".to_string(),
                repo,
                "--base".to_string(),
                option_str(options, "base", "HEAD")?,
                "--reviewer".to_string(),
                option_str(options, "reviewer", "security")?,
                "--objective".to_string(),
                option_str(options, "objective", "security")?,
                "--prompt-preview".to_string(),
                "--json".to_string(),
            ];
            if let Some(provider) = option_optional_str(options, "provider")? {
                args.extend(["--provider".to_string(), provider]);
            }
            if let Some(model) = option_optional_str(options, "model")? {
                args.extend(["--model".to_string(), model]);
            }
            Ok(PluginCommandPlan::Single(args))
        }
        "hooks-status" => plugin_hooks_plan(options, "status"),
        "hooks-install" => plugin_hooks_plan(options, "install"),
        "index" => {
            let mut args = vec!["index".to_string(), "--repo".to_string(), repo];
            match option_str(options, "mode", "changed")?.as_str() {
                "all" => args.push("--all".to_string()),
                "changed" => args.push("--changed".to_string()),
                other => return Err(anyhow!("unsupported Sober index mode: {other}")),
            }
            args.push("--json".to_string());
            Ok(PluginCommandPlan::Single(args))
        }
        "forge-doctor" => Ok(PluginCommandPlan::Single(vec![
            "forge".to_string(),
            "doctor".to_string(),
            "--repo".to_string(),
            repo,
            "--json".to_string(),
        ])),
        other => Err(anyhow!("unsupported Sober action for plugin: {other}")),
    }
}

fn plugin_hooks_plan(options: &Value, method: &str) -> Result<PluginCommandPlan> {
    if let Some(hook) = option_optional_str(options, "hook")? {
        validate_hook_name(&hook)?;
        return Ok(PluginCommandPlan::Single(vec![
            "hooks".to_string(),
            method.to_string(),
            hook,
            "--json".to_string(),
        ]));
    }

    Ok(PluginCommandPlan::Aggregate {
        key: "hooks",
        commands: ["pre-commit", "pre-push"]
            .into_iter()
            .map(|hook| {
                vec![
                    "hooks".to_string(),
                    method.to_string(),
                    hook.to_string(),
                    "--json".to_string(),
                ]
            })
            .collect(),
    })
}

fn validate_hook_name(hook: &str) -> Result<()> {
    match hook {
        "pre-commit" | "pre-push" => Ok(()),
        other => Err(anyhow!("unsupported Sober hook for plugin: {other}")),
    }
}

fn run_json(repo_root: &Path, args: &[String]) -> Result<Value, String> {
    let output = output(repo_root, args).map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format_sober_failure(&output));
    }
    serde_json::from_slice(&output.stdout).map_err(|e| {
        format!(
            "sober returned invalid JSON for `{}`: {e}",
            args.join(" ")
        )
    })
}

fn format_sober_failure(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    match (stdout.is_empty(), stderr.is_empty()) {
        (false, false) => format!("{stdout}\n{stderr}"),
        (false, true) => stdout,
        (true, false) => stderr,
        (true, true) => format!("sober exited with {}", output.status),
    }
}

fn failed(error: String) -> SoberInvocationResult {
    SoberInvocationResult {
        ok: false,
        data: json!({}),
        error: Some(error),
    }
}

fn option_str(options: &Value, name: &str, default: &str) -> Result<String> {
    option_optional_str(options, name).map(|value| value.unwrap_or_else(|| default.to_string()))
}

fn option_optional_str(options: &Value, name: &str) -> Result<Option<String>> {
    match options.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(anyhow!("Sober option `{name}` must be a string")),
    }
}

fn option_bool(options: &Value, name: &str, default: bool) -> bool {
    options.get(name).and_then(Value::as_bool).unwrap_or(default)
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

    #[test]
    fn plugin_rejects_unknown_action() {
        let invocation = SoberInvocation {
            action: "exec".to_string(),
            options: json!({}),
        };
        let err = plugin_command_plan(Path::new("/repo"), &invocation)
            .unwrap_err()
            .to_string();

        assert!(err.contains("unsupported Sober action"));
    }

    #[test]
    fn plugin_rejects_unknown_hook() {
        let invocation = SoberInvocation {
            action: "hooks-status".to_string(),
            options: json!({ "hook": "post-checkout" }),
        };
        let err = plugin_command_plan(Path::new("/repo"), &invocation)
            .unwrap_err()
            .to_string();

        assert!(err.contains("unsupported Sober hook"));
    }

    #[test]
    fn plugin_builds_review_preview_args() {
        let invocation = SoberInvocation {
            action: "review-preview".to_string(),
            options: json!({
                "base": "origin/main",
                "provider": "kimi-coding",
                "model": "kimi-k2.6",
            }),
        };
        let plan = plugin_command_plan(Path::new("/repo"), &invocation).unwrap();
        let PluginCommandPlan::Single(args) = plan else {
            panic!("expected single command")
        };

        assert_eq!(args[0], "review");
        assert!(args.contains(&"--prompt-preview".to_string()));
        assert!(args.contains(&"--json".to_string()));
        assert!(args
            .windows(2)
            .any(|pair| pair[0] == "--base" && pair[1] == "origin/main"));
        assert!(args
            .windows(2)
            .any(|pair| pair[0] == "--model" && pair[1] == "kimi-k2.6"));
    }
}
