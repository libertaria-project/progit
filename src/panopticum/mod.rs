// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2025 Markus Maiwald

//! Panopticum Integration Module
//!
//! Transforms ProGit into an Infrastructure Cockpit when PANOPTICUM.kdl is detected.
//! Integrates with the `panoctl` binary for validation, planning, and deployment.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐      ┌──────────────────┐
//! │   ProGit TUI    │ ◄─── │  PanoRunner      │
//! │   (main loop)   │      │  (background)    │
//! └────────┬────────┘      └────────┬─────────┘
//!          │ channel                │ thread
//!          ▼                        ▼
//!    PanoStatus updates      panoctl subprocess
//! ```
//!
//! All subprocess execution is **non-blocking** to prevent TUI freeze.

use anyhow::{Context, Result};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

/// Default binary name
const PANOCTL_BINARY: &str = "panoctl";

/// Status of panopticum operations (for TUI display)
#[derive(Debug, Clone)]
pub enum PanoStatus {
    /// No operation running
    Idle,
    /// Operation in progress with message
    Running(String),
    /// Success result
    Success(String),
    /// Error result
    Error(String),
    /// Output line (for streaming to console view)
    OutputLine(String),
}

/// Events sent from background thread to TUI
#[derive(Debug, Clone)]
pub enum PanoEvent {
    /// Status update
    Status(PanoStatus),
    /// Validation completed
    ValidationComplete { success: bool, message: String },
    /// Plan completed
    PlanComplete { success: bool, output: String },
    /// Apply completed
    ApplyComplete { success: bool, output: String },
}

/// Check if repository contains PANOPTICUM.kdl
pub fn is_panopticum_repo(root: &Path) -> bool {
    root.join("PANOPTICUM.kdl").exists()
}

/// Check if PANOPTICUM.kdl has staged changes (for pre-commit validation)
pub fn has_staged_panopticum_changes(root: &Path) -> Result<bool> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["diff", "--cached", "--name-only"])
        .output()
        .context("Failed to run git diff")?;

    let files = String::from_utf8_lossy(&output.stdout);
    Ok(files.lines().any(|f| f == "PANOPTICUM.kdl"))
}

/// Check if panoctl binary is available
pub fn is_panoctl_available(binary_path: Option<&str>) -> bool {
    let binary = binary_path.unwrap_or(PANOCTL_BINARY);
    Command::new(binary)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Get panoctl binary path (from config or fallback to PATH)
#[allow(dead_code)]
fn get_binary_path(custom_path: Option<&str>) -> String {
    custom_path
        .map(|s| s.to_string())
        .unwrap_or_else(|| PANOCTL_BINARY.to_string())
}

/// Spawn validation in background thread (non-blocking)
///
/// Results are sent back via the channel.
pub fn spawn_validate(root: PathBuf, binary_path: Option<String>, sender: Sender<PanoEvent>) {
    thread::spawn(move || {
        let _ = sender.send(PanoEvent::Status(PanoStatus::Running(
            "🔱 Validating policy...".into(),
        )));

        let binary = binary_path.as_deref().unwrap_or(PANOCTL_BINARY);
        let result = Command::new(binary)
            .current_dir(&root)
            .args(["validate", "PANOPTICUM.kdl"])
            .output();

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let message = if stdout.trim().is_empty() {
                    stderr.trim().to_string()
                } else {
                    stdout.trim().to_string()
                };

                let _ = sender.send(PanoEvent::ValidationComplete {
                    success: output.status.success(),
                    message,
                });
            }
            Err(e) => {
                let _ = sender.send(PanoEvent::ValidationComplete {
                    success: false,
                    message: format!("Failed to run panoctl: {}", e),
                });
            }
        }
    });
}

/// Spawn plan command in background thread with stdout streaming
///
/// Each line of output is sent as a `PanoEvent::Status(OutputLine(...))`.
pub fn spawn_plan(
    root: PathBuf,
    env: String,
    binary_path: Option<String>,
    sender: Sender<PanoEvent>,
) {
    thread::spawn(move || {
        let _ = sender.send(PanoEvent::Status(PanoStatus::Running(format!(
            "🔱 Planning {} environment...",
            env
        ))));

        let binary = binary_path.as_deref().unwrap_or(PANOCTL_BINARY);
        let child = Command::new(binary)
            .current_dir(&root)
            .args(["plan", "--env", &env])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        match child {
            Ok(mut child) => {
                let mut full_output = String::new();

                // Stream stdout
                if let Some(stdout) = child.stdout.take() {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines().flatten() {
                        let _ =
                            sender.send(PanoEvent::Status(PanoStatus::OutputLine(line.clone())));
                        full_output.push_str(&line);
                        full_output.push('\n');
                    }
                }

                // Capture stderr
                if let Some(stderr) = child.stderr.take() {
                    let reader = BufReader::new(stderr);
                    for line in reader.lines().flatten() {
                        let _ = sender.send(PanoEvent::Status(PanoStatus::OutputLine(format!(
                            "[ERR] {}",
                            line
                        ))));
                        full_output.push_str(&line);
                        full_output.push('\n');
                    }
                }

                let status = child.wait().ok().map(|s| s.success()).unwrap_or(false);
                let _ = sender.send(PanoEvent::PlanComplete {
                    success: status,
                    output: full_output,
                });
            }
            Err(e) => {
                let _ = sender.send(PanoEvent::PlanComplete {
                    success: false,
                    output: format!("Failed to spawn panoctl: {}", e),
                });
            }
        }
    });
}

/// Spawn apply command in background thread
///
/// **WARNING**: This executes infrastructure changes! Use with caution.
pub fn spawn_apply(
    root: PathBuf,
    env: String,
    auto_approve: bool,
    binary_path: Option<String>,
    sender: Sender<PanoEvent>,
) {
    thread::spawn(move || {
        let _ = sender.send(PanoEvent::Status(PanoStatus::Running(format!(
            "🔱 Applying {} environment...",
            env
        ))));

        let binary = binary_path.as_deref().unwrap_or(PANOCTL_BINARY);
        let mut args = vec!["apply", "--env", &env];
        if auto_approve {
            args.push("--auto-approve");
        }

        let child = Command::new(binary)
            .current_dir(&root)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        match child {
            Ok(mut child) => {
                let mut full_output = String::new();

                if let Some(stdout) = child.stdout.take() {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines().flatten() {
                        let _ =
                            sender.send(PanoEvent::Status(PanoStatus::OutputLine(line.clone())));
                        full_output.push_str(&line);
                        full_output.push('\n');
                    }
                }

                if let Some(stderr) = child.stderr.take() {
                    let reader = BufReader::new(stderr);
                    for line in reader.lines().flatten() {
                        let _ = sender.send(PanoEvent::Status(PanoStatus::OutputLine(format!(
                            "[ERR] {}",
                            line
                        ))));
                        full_output.push_str(&line);
                        full_output.push('\n');
                    }
                }

                let status = child.wait().ok().map(|s| s.success()).unwrap_or(false);
                let _ = sender.send(PanoEvent::ApplyComplete {
                    success: status,
                    output: full_output,
                });
            }
            Err(e) => {
                let _ = sender.send(PanoEvent::ApplyComplete {
                    success: false,
                    output: format!("Failed to spawn panoctl: {}", e),
                });
            }
        }
    });
}

/// Create a new channel for panopticum events
pub fn create_event_channel() -> (Sender<PanoEvent>, Receiver<PanoEvent>) {
    mpsc::channel()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_is_panopticum_repo_true() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("PANOPTICUM.kdl"), "network \"test\" {}").unwrap();
        assert!(is_panopticum_repo(dir.path()));
    }

    #[test]
    fn test_is_panopticum_repo_false() {
        let dir = tempdir().unwrap();
        assert!(!is_panopticum_repo(dir.path()));
    }

    #[test]
    fn test_get_binary_path_default() {
        assert_eq!(get_binary_path(None), "panoctl");
    }

    #[test]
    fn test_get_binary_path_custom() {
        assert_eq!(
            get_binary_path(Some("/opt/bin/panoctl")),
            "/opt/bin/panoctl"
        );
    }

    #[test]
    fn test_channel_creation() {
        let (sender, _receiver) = create_event_channel();
        // Should not panic
        let _ = sender.send(PanoEvent::Status(PanoStatus::Idle));
    }
}
