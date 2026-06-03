// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2025 Markus Maiwald

//! Citadel Integration Module
//!
//! Transforms ProGit into an Infrastructure Cockpit when CITADEL.kdl is detected.
//! Integrates with the `citadel` binary for validation, planning, and deployment.
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
//!    CitadelStatus updates      citadel subprocess
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
const PANOCTL_BINARY: &str = "citadel";

/// Status of citadel operations (for TUI display)
#[derive(Debug, Clone)]
pub enum CitadelStatus {
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
pub enum CitadelEvent {
    /// Status update
    Status(CitadelStatus),
    /// Validation completed
    ValidationComplete { success: bool, message: String },
    /// Plan completed
    PlanComplete { success: bool, output: String },
    /// Apply completed
    ApplyComplete { success: bool, output: String },
}

/// Check if repository contains CITADEL.kdl
pub fn is_citadel_repo(root: &Path) -> bool {
    root.join("CITADEL.kdl").exists()
}

/// Check if CITADEL.kdl has staged changes (for pre-commit validation)
pub fn has_staged_citadel_changes(root: &Path) -> Result<bool> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["diff", "--cached", "--name-only"])
        .output()
        .context("Failed to run git diff")?;

    let files = String::from_utf8_lossy(&output.stdout);
    Ok(files.lines().any(|f| f == "CITADEL.kdl"))
}

/// Check if citadel binary is available
pub fn is_citadel_available(binary_path: Option<&str>) -> bool {
    let binary = binary_path.unwrap_or(PANOCTL_BINARY);
    Command::new(binary)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Get citadel binary path (from config or fallback to PATH)
#[allow(dead_code)]
fn get_binary_path(custom_path: Option<&str>) -> String {
    custom_path
        .map(|s| s.to_string())
        .unwrap_or_else(|| PANOCTL_BINARY.to_string())
}

/// Spawn validation in background thread (non-blocking)
///
/// Results are sent back via the channel.
pub fn spawn_validate(root: PathBuf, binary_path: Option<String>, sender: Sender<CitadelEvent>) {
    thread::spawn(move || {
        let _ = sender.send(CitadelEvent::Status(CitadelStatus::Running(
            "🔱 Validating policy...".into(),
        )));

        let binary = binary_path.as_deref().unwrap_or(PANOCTL_BINARY);
        let result = Command::new(binary)
            .current_dir(&root)
            .args(["validate", "CITADEL.kdl"])
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

                let _ = sender.send(CitadelEvent::ValidationComplete {
                    success: output.status.success(),
                    message,
                });
            }
            Err(e) => {
                let _ = sender.send(CitadelEvent::ValidationComplete {
                    success: false,
                    message: format!("Failed to run citadel: {}", e),
                });
            }
        }
    });
}

/// Spawn plan command in background thread with stdout streaming
///
/// Each line of output is sent as a `CitadelEvent::Status(OutputLine(...))`.
pub fn spawn_plan(
    root: PathBuf,
    env: String,
    binary_path: Option<String>,
    sender: Sender<CitadelEvent>,
) {
    thread::spawn(move || {
        let _ = sender.send(CitadelEvent::Status(CitadelStatus::Running(format!(
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
                            sender.send(CitadelEvent::Status(CitadelStatus::OutputLine(line.clone())));
                        full_output.push_str(&line);
                        full_output.push('\n');
                    }
                }

                // Capture stderr
                if let Some(stderr) = child.stderr.take() {
                    let reader = BufReader::new(stderr);
                    for line in reader.lines().flatten() {
                        let _ = sender.send(CitadelEvent::Status(CitadelStatus::OutputLine(format!(
                            "[ERR] {}",
                            line
                        ))));
                        full_output.push_str(&line);
                        full_output.push('\n');
                    }
                }

                let status = child.wait().ok().map(|s| s.success()).unwrap_or(false);
                let _ = sender.send(CitadelEvent::PlanComplete {
                    success: status,
                    output: full_output,
                });
            }
            Err(e) => {
                let _ = sender.send(CitadelEvent::PlanComplete {
                    success: false,
                    output: format!("Failed to spawn citadel: {}", e),
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
    sender: Sender<CitadelEvent>,
) {
    thread::spawn(move || {
        let _ = sender.send(CitadelEvent::Status(CitadelStatus::Running(format!(
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
                            sender.send(CitadelEvent::Status(CitadelStatus::OutputLine(line.clone())));
                        full_output.push_str(&line);
                        full_output.push('\n');
                    }
                }

                if let Some(stderr) = child.stderr.take() {
                    let reader = BufReader::new(stderr);
                    for line in reader.lines().flatten() {
                        let _ = sender.send(CitadelEvent::Status(CitadelStatus::OutputLine(format!(
                            "[ERR] {}",
                            line
                        ))));
                        full_output.push_str(&line);
                        full_output.push('\n');
                    }
                }

                let status = child.wait().ok().map(|s| s.success()).unwrap_or(false);
                let _ = sender.send(CitadelEvent::ApplyComplete {
                    success: status,
                    output: full_output,
                });
            }
            Err(e) => {
                let _ = sender.send(CitadelEvent::ApplyComplete {
                    success: false,
                    output: format!("Failed to spawn citadel: {}", e),
                });
            }
        }
    });
}

/// Create a new channel for citadel events
pub fn create_event_channel() -> (Sender<CitadelEvent>, Receiver<CitadelEvent>) {
    mpsc::channel()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_is_citadel_repo_true() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("CITADEL.kdl"), "network \"test\" {}").unwrap();
        assert!(is_citadel_repo(dir.path()));
    }

    #[test]
    fn test_is_citadel_repo_false() {
        let dir = tempdir().unwrap();
        assert!(!is_citadel_repo(dir.path()));
    }

    #[test]
    fn test_get_binary_path_default() {
        assert_eq!(get_binary_path(None), "citadel");
    }

    #[test]
    fn test_get_binary_path_custom() {
        assert_eq!(
            get_binary_path(Some("/opt/bin/citadel")),
            "/opt/bin/citadel"
        );
    }

    #[test]
    fn test_channel_creation() {
        let (sender, _receiver) = create_event_channel();
        // Should not panic
        let _ = sender.send(CitadelEvent::Status(CitadelStatus::Idle));
    }
}
