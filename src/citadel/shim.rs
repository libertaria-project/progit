// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2025 Markus Maiwald

//! Citadel Command Shim
//!
//! Plugin-like boundary for Citadel commands. This module exposes a clean
//! `execute()` interface that mirrors what a future standalone plugin would
//! provide, while still being compiled into core.
//!
//! When the plugin SDK gains async jobs, streaming modals, and top-level
//! command registration, this shim becomes the migration point — the host
//! calls stay identical, only the implementation moves to a plugin.

use crate::command::CommandAction;
use crate::tui::app::App;

/// Execute a Citadel subcommand through the shim.
///
/// `args` is the slice after the command name, e.g. `["validate"]` or `["plan", "devnet"]`.
pub fn execute(app: &mut App, args: &[&str]) -> CommandAction {
    // Check if we're in a Citadel repo
    if !app.is_citadel_repo {
        return CommandAction::Error(
            "Not a Citadel repo (no CITADEL.kdl found)".to_string(),
        );
    }

    // Check if citadel binary is available
    if !crate::citadel::is_citadel_available(app.citadel_binary_path.as_deref()) {
        return CommandAction::Error(
            "citadel binary not found in PATH. Install citadel first.".to_string(),
        );
    }

    if args.is_empty() {
        return CommandAction::Error(
            "Usage: :citadel <validate|plan|apply> [env]".to_string(),
        );
    }

    // Get or create event channel
    if app.citadel_event_tx.is_none() {
        let (tx, rx) = crate::citadel::create_event_channel();
        app.citadel_event_tx = Some(tx);
        app.citadel_event_rx = Some(rx);
    }

    // SAFETY: set in the is_none block immediately above
    let sender = app.citadel_event_tx.clone().unwrap();
    let repo_path = app.repo_path.clone();
    let binary_path = app.citadel_binary_path.clone();

    match args[0] {
        "validate" => {
            app.citadel_output.clear();
            app.citadel_status =
                crate::citadel::CitadelStatus::Running("Validating...".into());
            crate::citadel::spawn_validate(repo_path, binary_path, sender);
            CommandAction::Status("🔱 Validation started...".to_string())
        }
        "plan" => {
            let env = args.get(1).unwrap_or(&"devnet").to_string();
            app.citadel_output.clear();
            app.show_citadel_log = true; // Auto-open modal
            app.citadel_status =
                crate::citadel::CitadelStatus::Running(format!("Planning {}...", env));
            crate::citadel::spawn_plan(repo_path, env, binary_path, sender);
            CommandAction::Status("🔱 Plan started... Log viewer opened.".to_string())
        }
        "apply" => {
            CommandAction::Error(
                "⚠️ Apply disabled in command mode for safety. Use dedicated Ops interface."
                    .to_string(),
            )
        }
        "status" => {
            match &app.citadel_status {
                crate::citadel::CitadelStatus::Idle => {
                    CommandAction::Status("🔱 Citadel: Idle".to_string())
                }
                crate::citadel::CitadelStatus::Running(msg) => {
                    CommandAction::Status(format!("🔱 {}", msg))
                }
                crate::citadel::CitadelStatus::Success(msg) => {
                    CommandAction::Status(format!("✓ {}", msg))
                }
                crate::citadel::CitadelStatus::Error(msg) => {
                    CommandAction::Error(format!("✗ {}", msg))
                }
                crate::citadel::CitadelStatus::OutputLine(line) => {
                    CommandAction::Status(line.clone())
                }
            }
        }
        _ => CommandAction::Error(format!(
            "Unknown citadel command: {}. Use validate|plan|status",
            args[0]
        )),
    }
}
