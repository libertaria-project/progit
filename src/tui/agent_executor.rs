// Helper function to execute agent action (placed at end of input.rs file)

use crate::tui::app::{AgentMode, App};
use crate::tui::widget_agent_menu::AgentAction;

/// Execute an agent action on the currently selected virtual branch (patch mode)
pub fn execute_agent_action(app: &mut App, action: AgentAction) {
    app.agent_mode = AgentMode::Patch;

    let mut branch_info = None;

    // Get branch info
    if let Some(manager) = &app.vbranch_manager {
        if let Some(branch) = manager.list().get(app.vbranch_selected) {
            branch_info = Some((branch.id.clone(), branch.name.clone()));
        }
    }

    if let Some((branch_id, branch_name)) = branch_info {
        app.set_status(format!(
            "🤖 {} for {}...",
            action.display_name(),
            branch_name
        ));

        if app.agent_event_tx.is_some() {
            use crate::agent::ollama::OllamaClient;
            use crate::agent::{AgentClient, AgentRequest};

            // SAFETY: guarded by is_some() check on the enclosing if
            let tx = app.agent_event_tx.clone().unwrap();
            let session_id = branch_id.clone();
            let project_root =
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

            // Gather context
            let context_res = if let Some(manager) = &app.vbranch_manager {
                if let Some(branch) = manager.get(&branch_id) {
                    use crate::agent::context::gather_context;
                    gather_context(branch, &project_root).ok()
                } else {
                    None
                }
            } else {
                None
            };

            // Get action-specific prompt
            let user_prompt = action.prompt_template().to_string();
            let system_prompt = action.system_prompt().to_string();

            std::thread::spawn(move || {
                let client = OllamaClient::default();

                let mut prompt = String::new();
                if let Some(ctx) = context_res {
                    prompt.push_str(&ctx.to_prompt_string());
                    prompt.push_str("\n\n");
                }
                prompt.push_str(&user_prompt);

                let req = AgentRequest {
                    prompt,
                    system_prompt: Some(system_prompt),
                    ..Default::default()
                };

                if let Err(e) = client.stream_completion(req, tx.clone(), session_id.clone()) {
                    let _ = tx.send(crate::agent::AgentEvent::Error(session_id, e.to_string()));
                }
            });

            app.set_status(format!("🤖 Agent executing: {}", action.display_name()));
        } else {
            app.set_status("⚠️ Agent system not initialized");
        }
    } else {
        app.set_status("No branch selected");
    }
}

/// Run an offline AI code review on the currently displayed diff.
/// The review is read-only; nothing is auto-applied.
pub fn execute_diff_review(app: &mut App) {
    app.agent_mode = AgentMode::Review;

    let diff_text = get_raw_diff(app);
    if diff_text.trim().is_empty() {
        app.set_status("No diff to review — stage some changes first");
        app.agent_mode = AgentMode::Patch;
        return;
    }

    app.set_status("🤖 Ollama is reviewing your diff...");

    if let Some(tx) = app.agent_event_tx.clone() {
        use crate::agent::ollama::OllamaClient;
        use crate::agent::{AgentClient, AgentRequest};

        let session_id = format!("diff-review-{}", uuid::Uuid::new_v4());

        let prompt = format!(
            "Review the following git diff. Focus on:\n\
             1. Bugs, logic errors, and edge cases\n\
             2. Security issues (injection, overflow, leaks, auth)\n\
             3. Performance concerns (allocations, complexity)\n\
             4. Style and readability (naming, duplication)\n\
             5. Test coverage gaps\n\n\
             Be concise but thorough. Use markdown headings for each section.\n\n\
             ```diff\n{}\n```",
            diff_text
        );

        std::thread::spawn(move || {
            let client = OllamaClient::default();
            let req = AgentRequest {
                model: crate::agent::default_model(),
                prompt,
                system_prompt: Some(
                    "You are a senior staff engineer performing code review. \
                     Be direct, specific, and actionable. \
                     Never suggest changes you are not confident about.".to_string(),
                ),
                temperature: 0.3,
            };

            if let Err(e) = client.stream_completion(req, tx.clone(), session_id.clone()) {
                let _ = tx.send(crate::agent::AgentEvent::Error(session_id, e.to_string()));
            }
        });
    } else {
        app.set_status("⚠️ Agent system not initialized");
        app.agent_mode = AgentMode::Patch;
    }
}

/// Capture the raw diff text that the user is currently viewing.
fn get_raw_diff(app: &App) -> String {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    let args: Vec<&str> = match app.diff_state.as_ref().map(|s| &s.mode) {
        Some(crate::diff::DiffMode::Staged) => vec!["diff", "--cached"],
        Some(crate::diff::DiffMode::Custom(ref_name)) => {
            return std::process::Command::new("git")
                .args(["diff", ref_name])
                .current_dir(&cwd)
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .unwrap_or_default();
        }
        _ => vec!["diff"], // Unstaged
    };

    std::process::Command::new("git")
        .args(&args)
        .current_dir(&cwd)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}
