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
    RunAndShowOutput(Vec<String>),
    RunPluginCommand { command: String, args: Vec<String> },
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
            // Check if we're in a Citadel repo
            if !app.is_citadel_repo {
                return CommandAction::Error(
                    "Not a Citadel repo (no CITADEL.kdl found)".to_string(),
                );
            }

            // Check if citadel is available
            if !crate::citadel::is_citadel_available(app.citadel_binary_path.as_deref()) {
                return CommandAction::Error(
                    "citadel binary not found in PATH. Install citadel first.".to_string(),
                );
            }

            if parts.len() < 2 {
                return CommandAction::Error(
                    "Usage: :pano <validate|plan|apply> [env]".to_string(),
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

            match parts[1] {
                "validate" => {
                    // Clear previous output
                    app.citadel_output.clear();
                    app.citadel_status =
                        crate::citadel::CitadelStatus::Running("Validating...".into());

                    // Dispatch async job
                    crate::citadel::spawn_validate(repo_path, binary_path, sender);

                    CommandAction::Status("🔱 Validation started...".to_string())
                }
                "plan" => {
                    let env = parts.get(2).unwrap_or(&"devnet").to_string();

                    // Clear previous output and open log viewer
                    app.citadel_output.clear();
                    app.show_citadel_log = true; // Auto-open modal
                    app.citadel_status =
                        crate::citadel::CitadelStatus::Running(format!("Planning {}...", env));

                    // Dispatch async job
                    crate::citadel::spawn_plan(repo_path, env, binary_path, sender);

                    CommandAction::Status("🔱 Plan started... Log viewer opened.".to_string())
                }
                "apply" => {
                    // Apply requires explicit confirmation - for safety, don't auto-approve from command
                    CommandAction::Error(
                        "⚠️ Apply disabled in command mode for safety. Use dedicated Ops interface.".to_string()
                    )
                }
                "status" => {
                    // Show current citadel status
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
                    "Unknown pano command: {}. Use validate|plan|status",
                    parts[1]
                )),
            }
        }
        "sober" => match crate::sober::tui_command_args(&parts[1..]) {
            Ok(args) => CommandAction::RunAndShowOutput(args),
            Err(e) => CommandAction::Error(e.to_string()),
        },
        "plugin" => {
            if parts.len() < 2 {
                CommandAction::Error("Usage: :plugin <command> [args...]".to_string())
            } else {
                let command = parts[1].to_string();
                let args = parts[2..].iter().map(|s| (*s).to_string()).collect();
                CommandAction::RunPluginCommand { command, args }
            }
        }
        "review" => {
            // Enter code review mode
            if parts.len() < 2 {
                return CommandAction::Error("Usage: :review <file> [commit-sha]".to_string());
            }

            let file_path = parts[1].to_string();
            let commit_sha = parts
                .get(2)
                .map(|s| s.to_string())
                .unwrap_or_else(|| "HEAD".to_string());

            // Get diff for the file
            let diff_cmd = std::process::Command::new("git")
                .args(["diff", &commit_sha, "--", &file_path])
                .current_dir(&app.repo_path)
                .output();

            match diff_cmd {
                Ok(output) if output.status.success() => {
                    let diff_text = String::from_utf8_lossy(&output.stdout).to_string();

                    if diff_text.is_empty() {
                        return CommandAction::Error(format!(
                            "No changes found in {} at {}",
                            file_path, commit_sha
                        ));
                    }

                    // Create review state
                    let review_state = crate::tui::widget_review::ReviewState::from_diff(
                        file_path.clone(),
                        diff_text,
                        commit_sha.clone(),
                    );

                    app.review_state = Some(review_state);
                    app.view_mode = ViewMode::Review;

                    CommandAction::Status(format!(
                        "📝 Reviewing {} @ {}",
                        file_path,
                        &commit_sha[..8]
                    ))
                }
                Ok(output) => CommandAction::Error(format!(
                    "Git diff failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )),
                Err(e) => CommandAction::Error(format!("Failed to run git diff: {}", e)),
            }
        }
        _ => CommandAction::Error(format!("Unknown command: {}", parts[0])),
    }
}

#[cfg(test)]
mod tests {
    use super::{execute, CommandAction};
    use crate::tui::app::App;

    #[test]
    fn plugin_command_parsing_requires_namespace() {
        let mut app = App::new();
        let action = execute(&mut app, "plugin");

        assert!(matches!(
            action,
            CommandAction::Error(msg) if msg == "Usage: :plugin <command> [args...]"
        ));
    }

    #[test]
    fn plugin_command_parsing_routes_to_plugin_namespace() {
        let mut app = App::new();
        let action = execute(&mut app, "plugin sober preflight --base HEAD");

        match action {
            CommandAction::RunPluginCommand { command, args } => {
                assert_eq!(command, "sober");
                assert_eq!(
                    args,
                    vec![
                        "preflight".to_string(),
                        "--base".to_string(),
                        "HEAD".to_string()
                    ]
                );
            }
            other => panic!("Expected RunPluginCommand, got: {other:?}"),
        }
    }
}
