// Helper function to execute agent action (placed at end of input.rs file)

use crate::tui::app::App;
use crate::tui::widget_agent_menu::AgentAction;

/// Execute an agent action on the currently selected virtual branch
pub fn execute_agent_action(app: &mut App, action: AgentAction) {
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
