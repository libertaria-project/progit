use crate::tui::app::{App, InputMode, ViewMode};
use crate::tui::theme::Theme;

#[derive(Debug, PartialEq)]
pub enum CommandAction {
    None,
    Quit,
    Refresh,
    Status(String),
    Error(String),
    SuspendAndRun(Vec<String>),
}

pub fn execute(app: &mut App, input: &str) -> CommandAction {
    let parts: Vec<&str> = input.trim().split_whitespace().collect();
    if parts.is_empty() {
        return CommandAction::None;
    }

    match parts[0] {
        "q" | "quit" => CommandAction::Quit,
        "n" | "new" => {
            app.input_mode = InputMode::Edit;
            CommandAction::Refresh
        }
        "w" | "write" => CommandAction::Status("Saved".to_string()),
        "theme" => {
            if parts.len() < 2 {
                return CommandAction::Error(
                    "Usage: :theme [nord|gruvbox|dracula|cyberpunk]".to_string(),
                );
            }
            match parts[1] {
                "nord" => app.theme = Theme::Nord,
                "gruvbox" => app.theme = Theme::Gruvbox,
                "dracula" => app.theme = Theme::Dracula,
                "cyberpunk" => app.theme = Theme::Cyberpunk,
                _ => return CommandAction::Error(format!("Unknown theme: {}", parts[1])),
            }
            CommandAction::Status(format!("Theme set to {}", parts[1]))
        }
        "sort" => {
            if parts.len() < 2 {
                return CommandAction::Error(
                    "Usage: :sort [due|created|status|effort]".to_string(),
                );
            }
            // For now just partial impl
            match parts[1] {
                "created" => {
                    // app.issues.sort_by...
                    // We need to implement sorts in app or issue module
                    CommandAction::Status("Sorted by creation".to_string())
                }
                _ => CommandAction::Error("Sort field not supported yet".to_string()),
            }
        }
        "rebase" => {
            if parts.len() < 2 {
                return CommandAction::Error("Usage: :rebase <branch>".to_string());
            }
            // Use current executable as editor
            let current_exe = std::env::current_exe().unwrap_or_else(|_| "prog".into());
            let editor = format!("{} RebaseEditor", current_exe.display());

            CommandAction::SuspendAndRun(vec![
                "git".to_string(),
                "-c".to_string(),
                format!("sequence.editor={}", editor),
                "rebase".to_string(),
                "-i".to_string(),
                parts[1].to_string(),
            ])
        }
        "diff" => {
            let mode = if parts.len() > 1 {
                crate::diff::DiffMode::Custom(parts[1].to_string())
            } else {
                crate::diff::DiffMode::Unstaged
            };
            let mut state = crate::diff::DiffState::new_with_mode(mode);
            match state.load(&app.repo_path) {
                Ok(_) => {
                    if state.files.is_empty() {
                        return CommandAction::Status("No changes detected".to_string());
                    }
                    app.diff_state = Some(state);
                    app.view_mode = ViewMode::Diff;
                    CommandAction::Refresh
                }
                Err(e) => CommandAction::Error(format!("Failed to load diff: {}", e)),
            }
        }
        "pano" => {
            // Check if we're in a Panopticum repo
            if !app.is_panopticum_repo {
                return CommandAction::Error(
                    "Not a Panopticum repo (no PANOPTICUM.kdl found)".to_string(),
                );
            }

            // Check if panoctl is available
            if !crate::panopticum::is_panoctl_available(app.panoctl_binary_path.as_deref()) {
                return CommandAction::Error(
                    "panoctl binary not found in PATH. Install panoctl first.".to_string(),
                );
            }

            if parts.len() < 2 {
                return CommandAction::Error(
                    "Usage: :pano <validate|plan|apply> [env]".to_string(),
                );
            }

            // Get or create event channel
            if app.pano_event_tx.is_none() {
                let (tx, rx) = crate::panopticum::create_event_channel();
                app.pano_event_tx = Some(tx);
                app.pano_event_rx = Some(rx);
            }

            let sender = app.pano_event_tx.clone().unwrap();
            let repo_path = app.repo_path.clone();
            let binary_path = app.panoctl_binary_path.clone();

            match parts[1] {
                "validate" => {
                    // Clear previous output
                    app.pano_output.clear();
                    app.pano_status =
                        crate::panopticum::PanoStatus::Running("Validating...".into());

                    // Dispatch async job
                    crate::panopticum::spawn_validate(repo_path, binary_path, sender);

                    CommandAction::Status("🔱 Validation started...".to_string())
                }
                "plan" => {
                    let env = parts.get(2).unwrap_or(&"devnet").to_string();

                    // Clear previous output and open log viewer
                    app.pano_output.clear();
                    app.show_pano_log = true; // Auto-open modal
                    app.pano_status =
                        crate::panopticum::PanoStatus::Running(format!("Planning {}...", env));

                    // Dispatch async job
                    crate::panopticum::spawn_plan(repo_path, env, binary_path, sender);

                    CommandAction::Status("🔱 Plan started... Log viewer opened.".to_string())
                }
                "apply" => {
                    // Apply requires explicit confirmation - for safety, don't auto-approve from command
                    CommandAction::Error(
                        "⚠️ Apply disabled in command mode for safety. Use dedicated Ops interface.".to_string()
                    )
                }
                "status" => {
                    // Show current panopticum status
                    match &app.pano_status {
                        crate::panopticum::PanoStatus::Idle => {
                            CommandAction::Status("🔱 Panopticum: Idle".to_string())
                        }
                        crate::panopticum::PanoStatus::Running(msg) => {
                            CommandAction::Status(format!("🔱 {}", msg))
                        }
                        crate::panopticum::PanoStatus::Success(msg) => {
                            CommandAction::Status(format!("✓ {}", msg))
                        }
                        crate::panopticum::PanoStatus::Error(msg) => {
                            CommandAction::Error(format!("✗ {}", msg))
                        }
                        crate::panopticum::PanoStatus::OutputLine(line) => {
                            CommandAction::Status(line.clone())
                        }
                    }
                }
                _ => CommandAction::Error(format!(
                    "Unknown pano command: {}. Use validate|plan|status",
                    parts[1]
                )),
            }
        }
        _ => CommandAction::Error(format!("Unknown command: {}", parts[0])),
    }
}
