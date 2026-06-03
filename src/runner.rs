//! Runner - TUI startup and main application loop

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use crate::git::detect_repo;
use crate::issue::Issue;
use crate::storage::paths;
use crate::tui::{handle_key, handle_mouse, render, App, KeyAction, UIAreas};

/// Convert ProGit Issue to Plugin SDK Issue format
pub(crate) fn convert_issue_to_plugin(issue: &Issue) -> progit_plugin_sdk::prelude::Issue {
    progit_plugin_sdk::prelude::Issue {
        id: issue.id.clone(),
        title: issue.title.clone(),
        description: issue.description.clone(),
        status: issue.status.as_str().to_string(),
        tags: issue.tags.clone(),
        assignee: issue.assignee.clone(),
        effort: Some(issue.effort as u8),
        blocked: issue.blocked,
        created: issue.created.to_rfc3339(),
        updated: issue.updated.to_rfc3339(),
        due: issue.due.map(|d| d.to_rfc3339()),
        metadata: std::collections::HashMap::new(),
    }
}

pub(crate) fn start_tui() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let result = run_app(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Handle errors
    if let Err(err) = result {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

pub(crate) fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>) -> Result<()> {
    let mut app = App::new();

    // Determine paths
    let project_root = crate::workspace::find_project_root()?;
    let config_path = project_root.join(paths::config_file());

    // Run migration from KDL to JSON if needed
    match crate::storage::migrate::migrate_kdl_to_json(&project_root) {
        Ok(count) if count > 0 => {
            log::info!("✅ Migrated {} issues from KDL to JSON", count);
        }
        Err(e) => {
            log::warn!("⚠️ Migration failed: {}", e);
        }
        _ => {}
    }

    // Initialize storage engine
    let mut engine = crate::storage::engine::StorageEngine::new(&project_root);
    engine.load()?;

    // Load config & init provider
    if let Ok(config) = crate::storage::config::load_config(&config_path) {
        if let Some(sync_config) = config.sync {
            app.sync_provider_name = Some(sync_config.provider.clone());
            app.sync_config = Some(sync_config.clone());
            app.sync_provider = Some(crate::sync::create_provider_with_auth(
                sync_config,
                crate::sync::AuthMode::NonInteractive,
            ));
        }
        // Apply saved theme
        if let Some(theme_name) = config.theme {
            app.theme = match theme_name.as_str() {
                "nord" => crate::tui::Theme::Nord,
                "gruvbox" => crate::tui::Theme::Gruvbox,
                "dracula" => crate::tui::Theme::Dracula,
                "cyberpunk" => crate::tui::Theme::Cyberpunk,
                "vibe" => crate::tui::Theme::Vibe,
                _ => crate::tui::Theme::Nord,
            };
        }

        // Initialize style engine with configured styles
        app.theme_engine = crate::tui::style::ThemeEngine::new(&config.styles);

        // Validate styles and warn if needed
        if let Err(e) = app.theme_engine.validate() {
            app.set_status(format!("Style Config Error: {}", e));
        }
    }

    // Load data from engine
    app.load_issues(engine.issues().to_vec());
    app.load_mrs(engine.mrs().to_vec());

    // Detect git repository from current working directory
    let cwd = std::env::current_dir()?;
    app.repo_info = detect_repo(&cwd)?;

    // ─── Plugin Loading ────────────────────────────────────────────────────────
    // Load plugins from repo plugins/ (legacy project-scoped) and
    // .progit/plugins/ (default user-installed).
    {
        use progit_plugin_sdk::prelude::PluginContext;

        let context = PluginContext {
            repo_path: project_root.to_string_lossy().to_string(),
            user: std::env::var("USER").ok(),
            env: std::env::vars().collect(),
            config: std::collections::HashMap::new(),
        };

        let mut plugin_manager = crate::plugins::PluginManager::new(&project_root);

        // Load project-scoped plugins (legacy path)
        match plugin_manager.load_all(&context) {
            Ok(count) if count > 0 => {
                log::info!("🔌 Loaded {} repo plugin(s)", count);
            }
            Err(e) => {
                log::warn!("⚠️ Repo plugin loading failed: {}", e);
            }
            _ => {}
        }

        // Also load from .progit/plugins/ (user-installed)
        let user_plugins = project_root.join(".progit").join("plugins");
        if user_plugins.exists() {
            plugin_manager.load_from_dir(&user_plugins, &context);
        }

        app.plugin_manager = Some(plugin_manager);
        app.fuzzy_searcher.update_plugin_commands(&project_root);
    }

    // ─── Cross-Repo Fuzzy Scan ────────────────────────────────────────────────
    {
        let current_repo_name = project_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        let base_dir = project_root.parent().unwrap_or_else(|| std::path::Path::new("."));
        app.fuzzy_searcher
            .scan_cross_repo_issues(base_dir, current_repo_name);
    }

    // ─── Citadel Integration ───────────────────────────────────────────────
    app.repo_path = project_root.clone();
    app.is_citadel_repo = crate::citadel::is_citadel_repo(&project_root);

    if app.is_citadel_repo {
        log::info!("🔱 Citadel mode activated");
        // Check if citadel is available (lazy check - don't fail if missing)
        if !crate::citadel::is_citadel_available(None) {
            log::warn!("⚠️ citadel binary not found in PATH");
        }
        // Create event channel for async operations
        let (tx, rx) = crate::citadel::create_event_channel();
        app.citadel_event_tx = Some(tx);
        app.citadel_event_rx = Some(rx);
    }

    // Track UI areas for mouse events
    let mut ui_areas = UIAreas::default();

    // ─── Agent Integration ───────────────────────────────────────────────────
    let (tx, rx) = std::sync::mpsc::channel();
    app.agent_event_tx = Some(tx);
    app.agent_event_rx = Some(rx);

    loop {
        // ─── Citadel Event Polling ─────────────────────────────────────────
        // Check for async citadel results before rendering
        if let Some(rx) = app.citadel_event_rx.take() {
            while let Ok(event) = rx.try_recv() {
                match event {
                    crate::citadel::CitadelEvent::Status(status) => {
                        match &status {
                            crate::citadel::CitadelStatus::OutputLine(line) => {
                                app.citadel_output.push(line.clone());
                                // Show last line in status bar
                                if let Some(last) = app.citadel_output.last() {
                                    app.set_status(last.clone());
                                }
                            }
                            _ => app.citadel_status = status,
                        }
                    }
                    crate::citadel::CitadelEvent::ValidationComplete { success, message } => {
                        if success {
                            app.citadel_status =
                                crate::citadel::CitadelStatus::Success(message.clone());
                            app.set_status(format!("✓ {}", message));
                        } else {
                            app.citadel_status = crate::citadel::CitadelStatus::Error(message.clone());
                            app.set_status(format!("✗ {}", message));
                        }
                    }
                    crate::citadel::CitadelEvent::PlanComplete { success, output } => {
                        if success {
                            app.citadel_status =
                                crate::citadel::CitadelStatus::Success("Plan complete".into());
                            app.set_status("✓ Plan completed successfully");
                        } else {
                            app.citadel_status = crate::citadel::CitadelStatus::Error(output.clone());
                            app.set_status(format!("✗ Plan failed"));
                        }
                    }
                    crate::citadel::CitadelEvent::ApplyComplete { success, output } => {
                        if success {
                            app.citadel_status =
                                crate::citadel::CitadelStatus::Success("Apply complete".into());
                            app.set_status("✓ Apply completed successfully");
                        } else {
                            app.citadel_status = crate::citadel::CitadelStatus::Error(output.clone());
                            app.set_status(format!("✗ Apply failed"));
                        }
                    }
                }
            }
            // Put receiver back
            app.citadel_event_rx = Some(rx);
        }

        // ─── Agent Event Polling ──────────────────────────────────────────────
        if let Some(rx) = app.agent_event_rx.take() {
            while let Ok(event) = rx.try_recv() {
                use crate::agent::AgentEvent;
                match event {
                    AgentEvent::Started(id) => {
                        // Update session status to Thinking
                        // This would ideally map id back to a virtual branch
                        // For now we just show a status
                        app.set_status(format!("🤖 Agent started (Session {})", &id[..8]));
                    }
                    AgentEvent::Token(_id, _token) => {
                        // Streaming token - in future we append to a buffer
                        // For now, simple indicator
                        // app.set_status(format!("🤖 Typing... {}", token)); // Too noisy
                    }
                    AgentEvent::Completed(id, response) => {
                        use crate::tui::app::AgentMode;
                        match app.agent_mode {
                            AgentMode::Review => {
                                app.command_output = Some(crate::tui::app::CommandOutput {
                                    command: "AI Diff Review".to_string(),
                                    status: "Completed".to_string(),
                                    stdout: response,
                                    stderr: String::new(),
                                    success: true,
                                    title: Some(" 🤖 AI Diff Review ".to_string()),
                                });
                                app.command_output_scroll = 0;
                                app.input_mode = crate::tui::app::InputMode::CommandOutput;
                                app.agent_mode = AgentMode::Patch; // Reset
                                app.set_status("🤖 Review complete — Press Enter to close");
                            }
                            AgentMode::Patch => {
                                app.set_status("🤖 Agent finished! Applying changes...");

                                if let Some(manager) = &mut app.vbranch_manager {
                                    use crate::agent::ops::apply_agent_patch;
                                    match apply_agent_patch(manager, &id, &response) {
                                        Ok(count) => {
                                            app.set_status(format!(
                                                "✅ Agent applied {} new hunk(s)",
                                                count
                                            ));
                                        }
                                        Err(e) => {
                                            log::error!("Agent apply error: {}", e);
                                            app.set_status(format!(
                                                "❌ Failed to apply agent patch: {}",
                                                e
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    AgentEvent::Error(_id, err) => {
                        app.set_status(format!("⚠️ Agent error: {}", err));
                    }
                }
            }
            app.agent_event_rx = Some(rx);
        }

        // Draw
        terminal.draw(|frame| {
            ui_areas = render(frame, &mut app);
        })?;

        // Handle input
        if event::poll(std::time::Duration::from_millis(100))? {
            let action = match event::read()? {
                Event::Key(key) => handle_key(&mut app, key),
                Event::Mouse(mouse) => handle_mouse(&mut app, mouse, &ui_areas),
                _ => KeyAction::None,
            };

            match action {
                KeyAction::Quit => break,
                KeyAction::Save => {
                    // Sync app issues to engine and save
                    *engine.issues_mut() = app.issues.clone();
                    engine.save()?;
                }
                KeyAction::SaveTheme => {
                    // Save theme preference to config
                    let theme_name = match app.theme {
                        crate::tui::Theme::Nord => "nord",
                        crate::tui::Theme::Gruvbox => "gruvbox",
                        crate::tui::Theme::Dracula => "dracula",
                        crate::tui::Theme::Cyberpunk => "cyberpunk",
                        crate::tui::Theme::Vibe => "vibe",
                    };
                    let _ = crate::storage::save_theme(&config_path, theme_name);
                    app.set_status(format!("Theme: {}", theme_name));
                }
                KeyAction::CreateIssue(status) => {
                    // Create a new issue with optional status
                    let mut new_issue = Issue::new("New Issue");
                    if let Some(s) = status {
                        new_issue.status = s;
                    }
                    engine.upsert(new_issue.clone())?;
                    app.load_issues(engine.issues().to_vec());
                    app.set_status("Created new issue");

                    // Trigger plugin hook
                    if let Some(ref mut pm) = app.plugin_manager {
                        let plugin_issue = convert_issue_to_plugin(&new_issue);
                        pm.on_issue_created(&plugin_issue);
                    }
                }
                KeyAction::DeleteIssue => {
                    // Delete the selected issue based on view mode
                    let issue_id = match app.view_mode {
                        crate::tui::ViewMode::Dashboard => None,
                        crate::tui::ViewMode::List => app.selected_issue().map(|i| i.id.clone()),
                        crate::tui::ViewMode::Kanban => {
                            app.kanban_selected_issue().map(|i| i.id.clone())
                        }
                        crate::tui::ViewMode::Diff => None,
                        crate::tui::ViewMode::MRList => None,
                        crate::tui::ViewMode::Blame => None,
                        crate::tui::ViewMode::Lanes => None,
                        crate::tui::ViewMode::Review => None,
                    };

                    if let Some(id) = issue_id {
                        if engine.delete(&id)? {
                            app.load_issues(engine.issues().to_vec());
                            app.set_status("Issue deleted");

                            // Trigger plugin hook
                            if let Some(ref mut pm) = app.plugin_manager {
                                pm.on_issue_deleted(&id);
                            }
                        } else {
                            app.set_status("Failed to delete issue");
                        }
                    }
                }
                KeyAction::Sync => {
                    // Temporarily take provider to avoid borrow conflict
                    if let Some(provider) = app.sync_provider.take() {
                        let provider_name = app.sync_provider_name.as_deref().unwrap_or("remote");
                        app.set_status(format!("Syncing with {}...", provider_name));
                        terminal.draw(|frame| {
                            ui_areas = render(frame, &mut app);
                        })?;

                        // 1. PUSH
                        if let Err(e) = provider
                            .login()
                            .and_then(|_| provider.push(&mut app.issues))
                        {
                            app.set_remote_error_status(format!("Push failed: {}", e));
                        } else {
                            // Persist links after push
                            *engine.issues_mut() = app.issues.clone();
                            if let Err(e) = engine.save() {
                                app.set_status(format!("Save failed: {}", e));
                            } else {
                                // 2. DELETE MISSING
                                let _ = provider.delete_missing(&app.issues);

                                // 3. PULL ISSUES
                                match provider.pull() {
                                    Ok(remote_issues) => {
                                        let provider_name =
                                            app.sync_provider_name.as_deref().unwrap_or("gitlab");
                                        let merged = crate::sync::merge_issues(
                                            &app.issues,
                                            remote_issues,
                                            provider_name,
                                        );
                                        app.load_issues(merged.clone());
                                        *engine.issues_mut() = merged;
                                    }
                                    Err(e) => {
                                        app.set_remote_error_status(format!(
                                            "Issues pull failed: {}",
                                            e
                                        ));
                                    }
                                }

                                // 4. PULL MRS
                                match provider.list_mrs() {
                                    Ok(remote_mrs) => {
                                        let provider_name =
                                            app.sync_provider_name.as_deref().unwrap_or("gitlab");
                                        let merged = crate::sync::merge_mrs(
                                            &app.mr_list,
                                            remote_mrs,
                                            provider_name,
                                        );
                                        app.load_mrs(merged.clone());
                                        *engine.mrs_mut() = merged;
                                        app.set_status("Sync Complete (Issues & MRs)!");
                                    }
                                    Err(e) => {
                                        app.set_remote_error_status(format!(
                                            "MR pull failed: {}",
                                            e
                                        ));
                                    }
                                }

                                // Final Save
                                if let Err(e) = engine.save() {
                                    app.set_status(format!("Save failed: {}", e));
                                }
                            }
                        }
                        // Put provider back
                        app.sync_provider = Some(provider);
                    } else {
                        app.set_status("No sync provider configured.");
                    }
                }
                KeyAction::SwitchBranch(branch) => {
                    match crate::git::switch_branch(&cwd, &branch) {
                        Ok(_) => {
                            app.set_status(format!("Switched to branch: {}", branch));
                            // Refresh repo info
                            app.repo_info = detect_repo(&cwd)?;

                            // Reload issues from disk/engine
                            if let Err(e) = engine.load() {
                                app.set_status(format!("Reload failed: {}", e));
                            } else {
                                app.load_issues(engine.issues().to_vec());
                            }
                        }
                        Err(e) => app.set_status(format!("Failed to switch: {}", e)),
                    }
                }
                KeyAction::CreateBranch => {
                    // For now, simpler prompts. ideally we use an input box?
                    // MVP: Just auto-generate a name or prompt via "Edit" mode?
                    // We don't have a generic "InputBox" widget yet for random strings.
                    // We only have `edit_buffer` used for issues.
                    // Let's reuse `input_mode = InputMode::Edit` (legacy) or hijack search?
                    // Hack: Create "branch-TIMESTAMP" for MVP or ask user to implement Input Box properly later.
                    // Actually, let's use the USER REQUEST context: "creating a new one".
                    // I will stick to "create-feature" for now to check if it works.
                    // Wait, `Edit` mode in input.rs says "Legacy - redirect to detail view".
                    // Let's implement a real input dialog next time.
                    // For now, let's create "new-branch-<timestamp>"

                    let new_name = format!("branch-{}", chrono::Utc::now().timestamp());
                    match crate::git::create_branch(&cwd, &new_name) {
                        Ok(_) => {
                            app.set_status(format!("Created {}", new_name));
                            app.repo_info = detect_repo(&cwd)?;
                        }
                        Err(e) => app.set_status(format!("Failed to create: {}", e)),
                    }
                }
                KeyAction::CreateBranchNamed(name) => {
                    match crate::git::create_branch(&cwd, &name) {
                        Ok(_) => {
                            app.set_status(format!("Created & switched to: {}", name));
                            app.repo_info = detect_repo(&cwd)?;
                        }
                        Err(e) => app.set_status(format!("Failed to create '{}': {}", name, e)),
                    }
                }
                KeyAction::DeleteBranch(name) => match crate::git::delete_branch(&cwd, &name) {
                    Ok(_) => {
                        app.set_status(format!("Deleted branch: {}", name));
                        app.repo_info = detect_repo(&cwd)?;
                    }
                    Err(e) => app.set_status(format!("Failed to delete '{}': {}", name, e)),
                },
                KeyAction::Refresh | KeyAction::None => {}
                KeyAction::ToggleDebug => {
                    app.show_debug_console = !app.show_debug_console;
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
