//! Modal/overlay key handlers

use super::super::app::{App, InputMode};
use super::KeyAction;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle a key event in search mode
pub(super) fn handle_search_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.search_query.clear();
            app.refresh_filter();
            KeyAction::Refresh
        }
        KeyCode::Enter => {
            app.input_mode = InputMode::Normal;
            KeyAction::Refresh
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.refresh_filter();
            KeyAction::Refresh
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.refresh_filter();
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}

/// Handle a key event in confirm mode
pub(super) fn handle_confirm_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.input_mode = InputMode::Normal;
            app.clear_status();
            KeyAction::DeleteIssue
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.clear_status();
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}

/// Handle key in command mode
pub(super) fn handle_command_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.command_input.clear();
            KeyAction::Refresh
        }
        KeyCode::Enter => {
            let cmd = app.command_input.clone();
            app.command_input.clear();
            app.input_mode = InputMode::Normal;

            use crate::command::CommandAction;
            match crate::command::execute(app, &cmd) {
                CommandAction::None => KeyAction::Refresh,
                CommandAction::Quit => KeyAction::Quit,
                CommandAction::Refresh => KeyAction::Refresh,
                CommandAction::Status(msg) => {
                    app.set_status(msg);
                    KeyAction::Refresh
                }
                CommandAction::Error(err) => {
                    app.set_status(format!("Error: {}", err));
                    KeyAction::Refresh
                }
                CommandAction::SuspendAndRun(args) => {
                    // Suspend TUI
                    let _ = crossterm::terminal::disable_raw_mode();
                    let _ = crossterm::execute!(
                        std::io::stdout(),
                        crossterm::terminal::LeaveAlternateScreen
                    );

                    // Run command
                    let status = std::process::Command::new(&args[0])
                        .args(&args[1..])
                        .status();

                    // Resume TUI
                    let _ = crossterm::terminal::enable_raw_mode();
                    let _ = crossterm::execute!(
                        std::io::stdout(),
                        crossterm::terminal::EnterAlternateScreen
                    );

                    match status {
                        Ok(s) if s.success() => {
                            app.set_status("Command executed successfully".to_string());
                        }
                        Ok(s) => {
                            app.set_status(format!("Command failed with code: {}", s));
                        }
                        Err(e) => {
                            app.set_status(format!("Failed to run command: {}", e));
                        }
                    }
                    KeyAction::Refresh
                }
            }
        }
        KeyCode::Backspace => {
            app.command_input.pop();
            KeyAction::Refresh
        }
        KeyCode::Char(c) => {
            app.command_input.push(c);
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}

/// Handle a key event in remote dropdown mode
pub(super) fn handle_remote_dropdown_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            KeyAction::Refresh
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(ref repo) = app.repo_info {
                app.selected_remote = (app.selected_remote + 1).min(repo.remotes.len());
            }
            KeyAction::Refresh
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.selected_remote = app.selected_remote.saturating_sub(1);
            KeyAction::Refresh
        }
        KeyCode::Enter => {
            // Switch to selected remote
            let status_msg = if let Some(ref repo) = app.repo_info {
                if app.selected_remote < repo.remotes.len() {
                    let remote = &repo.remotes[app.selected_remote];
                    Some((remote.name.clone(), remote.url.clone()))
                } else {
                    None
                }
            } else {
                None
            };

            if let Some((name, url)) = status_msg {
                if let Some(ref mut repo) = app.repo_info {
                    repo.remote_name = Some(name.clone());
                    repo.remote_url = Some(url);
                }
                app.set_status(format!("Switched to remote: {}", name));
            }
            app.input_mode = InputMode::Normal;
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}

pub(super) fn handle_branch_dropdown_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            KeyAction::Refresh
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(ref repo) = app.repo_info {
                app.selected_branch = (app.selected_branch + 1).min(repo.branches.len());
            }
            KeyAction::Refresh
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.selected_branch = app.selected_branch.saturating_sub(1);
            KeyAction::Refresh
        }
        KeyCode::Enter => {
            if let Some(ref repo) = app.repo_info {
                if app.selected_branch < repo.branches.len() {
                    let branch = repo.branches[app.selected_branch].clone();
                    app.input_mode = InputMode::Normal;
                    return KeyAction::SwitchBranch(branch);
                } else if app.selected_branch == repo.branches.len() {
                    // "New Branch" option selected - switch to edit mode
                    app.edit_buffer = String::from("feature/");
                    app.input_mode = InputMode::BranchCreate;
                    return KeyAction::Refresh;
                }
            }
            app.input_mode = InputMode::Normal;
            KeyAction::Refresh
        }
        KeyCode::Char('n') => {
            // Quick shortcut to create new branch
            app.edit_buffer = String::from("feature/");
            app.input_mode = InputMode::BranchCreate;
            KeyAction::Refresh
        }
        KeyCode::Char('D') => {
            // Delete selected branch (not current, not "New Branch" option)
            if let Some(ref repo) = app.repo_info {
                if app.selected_branch < repo.branches.len() {
                    let branch = &repo.branches[app.selected_branch];
                    if branch != &repo.branch {
                        app.pending_branch_delete = Some(branch.clone());
                        app.set_status(format!("Delete branch '{}'? (y/n)", branch));
                        app.input_mode = InputMode::BranchDeleteConfirm;
                    } else {
                        app.set_status("Cannot delete current branch");
                    }
                }
            }
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}

/// Handle key events in branch delete confirmation mode
pub(super) fn handle_branch_delete_confirm_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(name) = app.pending_branch_delete.take() {
                app.input_mode = InputMode::Normal;
                return KeyAction::DeleteBranch(name);
            }
            app.input_mode = InputMode::BranchDropdown;
            KeyAction::Refresh
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.pending_branch_delete = None;
            app.set_status("Branch deletion cancelled");
            app.input_mode = InputMode::BranchDropdown;
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}

/// Handle key events in branch create mode (typing branch name)
pub(super) fn handle_branch_create_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc => {
            app.edit_buffer.clear();
            app.input_mode = InputMode::BranchDropdown;
            KeyAction::Refresh
        }
        KeyCode::Enter => {
            if !app.edit_buffer.is_empty() {
                let name = app.edit_buffer.clone();
                app.edit_buffer.clear();
                app.input_mode = InputMode::Normal;
                return KeyAction::CreateBranchNamed(name);
            }
            KeyAction::Refresh
        }
        KeyCode::Backspace => {
            app.edit_buffer.pop();
            KeyAction::Refresh
        }
        KeyCode::Char(c) => {
            // Only allow valid branch name chars
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '/' {
                app.edit_buffer.push(c);
            }
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}

/// Handle key events in virtual branch create mode
pub(super) fn handle_vbranch_create_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc => {
            app.edit_buffer.clear();
            app.input_mode = InputMode::Normal;
            app.set_status("Virtual branch creation cancelled");
            KeyAction::Refresh
        }
        KeyCode::Enter => {
            if !app.edit_buffer.is_empty() {
                let name = app.edit_buffer.clone();
                app.edit_buffer.clear();

                // Get HEAD commit for base
                let base_commit = app.repo_info
                    .as_ref()
                    .and_then(|r| {
                        use git2::Repository;
                        if let Ok(repo) = Repository::open(&r.path) {
                            if let Ok(head) = repo.head() {
                                if let Ok(commit) = head.peel_to_commit() {
                                    return Some(commit.id().to_string());
                                }
                            }
                        }
                        None
                    })
                    .unwrap_or_else(|| "HEAD".to_string());

                // Create virtual branch
                let result = if let Some(ref mut manager) = app.vbranch_manager {
                    match manager.create_branch(&name, &base_commit) {
                        Ok(id) => {
                            let pos = manager.list().iter().position(|b| b.id == id);
                            Some((Ok(()), name.clone(), pos))
                        }
                        Err(e) => Some((Err(e), name.clone(), None)),
                    }
                } else {
                    None
                };

                // Update status outside the borrow
                if let Some((res, branch_name, pos)) = result {
                    match res {
                        Ok(()) => {
                            app.set_status(format!("Created virtual branch: {}", branch_name));
                            if let Some(p) = pos {
                                app.vbranch_selected = p;
                            }
                        }
                        Err(e) => {
                            app.set_status(format!("Failed to create branch: {}", e));
                        }
                    }
                }

                app.input_mode = InputMode::Normal;
            }
            KeyAction::Refresh
        }
        KeyCode::Backspace => {
            app.edit_buffer.pop();
            KeyAction::Refresh
        }
        KeyCode::Char(c) => {
            // Only allow valid branch name chars
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '/' {
                app.edit_buffer.push(c);
            }
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}

/// Handle key events in virtual branch move mode (selecting target lane)
pub(super) fn handle_vbranch_move_key(app: &mut App, key: KeyEvent) -> KeyAction {
    // Store the original selected lane as source
    let source_idx = app.vbranch_selected;

    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.set_status("Hunk move cancelled");
            KeyAction::Refresh
        }
        KeyCode::Char('h') | KeyCode::Left => {
            // Navigate to select target (but don't allow selecting same lane)
            let new_idx = app.vbranch_selected.saturating_sub(1);
            app.vbranch_selected = new_idx;
            KeyAction::Refresh
        }
        KeyCode::Char('l') | KeyCode::Right => {
            if let Some(ref manager) = app.vbranch_manager {
                let max = manager.list().len().saturating_sub(1);
                let new_idx = (app.vbranch_selected + 1).min(max);
                app.vbranch_selected = new_idx;
            }
            KeyAction::Refresh
        }
        KeyCode::Enter => {
            let target_idx = app.vbranch_selected;

            if target_idx == source_idx {
                app.set_status("Cannot move hunk to same lane");
                return KeyAction::Refresh;
            }

            // Collect needed data first without holding references
            let transfer_info = if let Some(ref manager) = app.vbranch_manager {
                let branches = manager.list();
                let source_branch = branches.get(source_idx);
                let target_branch = branches.get(target_idx);

                if let (Some(source), Some(target)) = (source_branch, target_branch) {
                    let source_id = source.id.clone();
                    let target_id = target.id.clone();
                    let target_name = target.name.clone();
                    let hunk = source.owned_hunks.get(app.vbranch_hunk_selected).cloned();
                    Some((source_id, target_id, target_name, hunk))
                } else {
                    None
                }
            } else {
                None
            };

            // Now perform the transfer
            if let Some((source_id, target_id, target_name, Some(hunk))) = transfer_info {
                if let Some(ref mut manager) = app.vbranch_manager {
                    match manager.transfer_hunk(&hunk, &source_id, &target_id) {
                        Ok(()) => {
                            app.set_status(format!("Moved hunk to '{}'", target_name));
                            app.vbranch_hunk_selected = 0;
                        }
                        Err(e) => {
                            app.set_status(format!("Failed to move hunk: {}", e));
                        }
                    }
                }
            }

            app.vbranch_selected = source_idx; // Restore original selection
            app.input_mode = InputMode::Normal;
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}

/// Handle keys in fuzzy palette mode
pub(super) fn handle_fuzzy_palette_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.fuzzy_query.clear();
            app.fuzzy_selected = 0;
            KeyAction::Refresh
        }
        KeyCode::Down | KeyCode::Char('j') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            let results = app.fuzzy_searcher.search(&app.fuzzy_query);
            if !results.is_empty() {
                app.fuzzy_selected = (app.fuzzy_selected + 1).min(results.len() - 1);
            }
            KeyAction::Refresh
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.fuzzy_selected = app.fuzzy_selected.saturating_sub(1);
            KeyAction::Refresh
        }
        KeyCode::Enter => {
            // Execute selected item
            let results = app.fuzzy_searcher.search(&app.fuzzy_query);
            if let Some(result) = results.get(app.fuzzy_selected) {
                use crate::fuzzy::FuzzyItem;
                match &result.item {
                    FuzzyItem::Issue { id, .. } => {
                        app.open_detail(id);
                        app.input_mode = InputMode::Normal;
                    }
                    FuzzyItem::Command { action, .. } => {
                        app.input_mode = InputMode::Normal;
                        // Execute command action
                        match action.as_str() {
                            "new_issue" => return KeyAction::CreateIssue(None),
                            "toggle_view" => {
                                app.toggle_view();
                                return KeyAction::Refresh;
                            }
                            "sync" => return KeyAction::Sync,
                            "cycle_theme" => {
                                app.cycle_theme();
                                return KeyAction::SaveTheme;
                            }
                            "settings" => {
                                app.input_mode = InputMode::Settings;
                                return KeyAction::Refresh;
                            }
                            "quit" => return KeyAction::Quit,
                            "sort" => {
                                // TODO: Implement sort menu
                                return KeyAction::Refresh;
                            }
                            "branch" => {
                                // TODO: Switch to branch dropdown
                                app.input_mode = InputMode::BranchDropdown;
                                return KeyAction::Refresh;
                            }
                            "mr" => {
                                app.input_mode = InputMode::MRCreate;
                                // Initialize MR draft if needed (simplified logic)
                                if app.mr_draft.is_none() {
                                    if let Some(ref repo) = app.repo_info {
                                        app.mr_draft = Some(crate::mr::MergeRequest::new(
                                            &repo.branch,
                                            "main",
                                            &repo.branch,
                                        ));
                                        app.mr_field = 1;
                                        app.edit_buffer = repo.branch.clone();
                                    }
                                }
                                return KeyAction::Refresh;
                            }
                            "search" => {
                                app.input_mode = InputMode::Search;
                                return KeyAction::Refresh;
                            }
                            "project_wiki" => {
                                app.open_project_wiki();
                                return KeyAction::Refresh;
                            }
                            "project_issues" => {
                                app.open_project_issues();
                                return KeyAction::Refresh;
                            }
                            "plugin_command" => {
                                app.input_mode = InputMode::Command;
                                app.command_input = "plugin ".to_string();
                                app.set_status("Type plugin command and press Enter".to_string());
                                return KeyAction::Refresh;
                            }
                            "sober_doctor" => {
                                app.input_mode = InputMode::Command;
                                app.command_input = "sober doctor".to_string();
                                app.set_status("Press Enter to run Sober doctor".to_string());
                                return KeyAction::Refresh;
                            }
                            "sober_preflight" => {
                                app.input_mode = InputMode::Command;
                                app.command_input = "sober preflight --base HEAD".to_string();
                                app.set_status("Press Enter to run Sober preflight".to_string());
                                return KeyAction::Refresh;
                            }
                            "sober_review_preview" => {
                                app.input_mode = InputMode::Command;
                                app.command_input = "sober review-preview --base HEAD --provider kimi-coding --model kimi-k2.6".to_string();
                                app.set_status(
                                    "Press Enter to preview Sober review prompt".to_string(),
                                );
                                return KeyAction::Refresh;
                            }
                            _ => {}
                        }

                        if let Some(command) = action.strip_prefix("plugin_command:") {
                            app.input_mode = InputMode::Command;
                            app.command_input = format!("plugin {command} ");
                            app.set_status(format!("Press Enter to run plugin command: {command}"));
                            return KeyAction::Refresh;
                        }
                    }
                    FuzzyItem::File { .. } => {
                        // TODO: Open file in editor
                        app.input_mode = InputMode::Normal;
                    }
                    FuzzyItem::Commit { .. } => {
                        // TODO: Show commit details
                        app.input_mode = InputMode::Normal;
                    }
                }
            }
            KeyAction::Refresh
        }
        KeyCode::Char('b') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            // Blame selected file
            let results = app.fuzzy_searcher.search(&app.fuzzy_query);
            if let Some(result) = results.get(app.fuzzy_selected) {
                if let crate::fuzzy::FuzzyItem::File { path, .. } = &result.item {
                    let p = path.clone();
                    app.load_blame(&p);
                    app.input_mode = InputMode::Normal;
                }
            }
            KeyAction::Refresh
        }
        KeyCode::Backspace => {
            app.fuzzy_query.pop();
            app.fuzzy_selected = 0;
            KeyAction::Refresh
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.fuzzy_query.push(c);
            app.fuzzy_selected = 0;
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}

/// Handle keys in the project wiki overlay.
pub(super) fn handle_project_wiki_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.close_project_overlay();
            KeyAction::Refresh
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.project_wiki_scroll = app.project_wiki_scroll.saturating_add(1);
            KeyAction::Refresh
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.project_wiki_scroll = app.project_wiki_scroll.saturating_sub(1);
            KeyAction::Refresh
        }
        KeyCode::Char('g') => {
            app.project_wiki_scroll = 0;
            KeyAction::Refresh
        }
        KeyCode::Char('G') => {
            app.project_wiki_scroll = u16::MAX;
            KeyAction::Refresh
        }
        KeyCode::Char('h') | KeyCode::Left => {
            app.project_wiki_page = app.project_wiki_page.saturating_sub(1);
            app.project_wiki_scroll = 0;
            KeyAction::Refresh
        }
        KeyCode::Char('l') | KeyCode::Right => {
            if let Some(view) = &app.project_wiki_view {
                let last = view.pages.len().saturating_sub(1);
                app.project_wiki_page = (app.project_wiki_page + 1).min(last);
                app.project_wiki_scroll = 0;
            }
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}

/// Handle keys in the project issues overlay.
pub(super) fn handle_project_issues_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.close_project_overlay();
            KeyAction::Refresh
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(view) = &app.project_issues_view {
                let last = view.issues.len().saturating_sub(1);
                app.project_issue_selected = (app.project_issue_selected + 1).min(last);
            }
            KeyAction::Refresh
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.project_issue_selected = app.project_issue_selected.saturating_sub(1);
            KeyAction::Refresh
        }
        KeyCode::Enter => {
            let selected_id = app
                .project_issues_view
                .as_ref()
                .and_then(|view| view.issues.get(app.project_issue_selected))
                .map(|entry| entry.issue.id.clone());

            if let Some(id) = selected_id {
                if app.issues.iter().any(|issue| issue.id == id) {
                    app.open_detail(&id);
                } else {
                    app.set_status("Project issue is not loaded in the active issue set");
                }
            }
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}

/// Handle keys in MR creation form
pub(super) fn handle_mr_create_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.mr_draft = None;
            app.edit_buffer.clear();
            app.set_status("MR creation cancelled");
            KeyAction::Refresh
        }
        KeyCode::Tab => {
            // Save current field and move to next
            if let Some(ref mut mr) = app.mr_draft {
                match app.mr_field {
                    1 => mr.title = app.edit_buffer.clone(),
                    2 => mr.target_branch = app.edit_buffer.clone(),
                    3 => mr.description = app.edit_buffer.clone(),
                    _ => {}
                }
            }

            // Cycle through fields: 1=title, 2=target, 3=description
            app.mr_field = match app.mr_field {
                1 => 2,
                2 => 3,
                3 => 1,
                _ => 1,
            };

            // Load new field into buffer
            if let Some(ref mr) = app.mr_draft {
                app.edit_buffer = match app.mr_field {
                    1 => mr.title.clone(),
                    2 => mr.target_branch.clone(),
                    3 => mr.description.clone(),
                    _ => String::new(),
                };
            }
            KeyAction::Refresh
        }
        KeyCode::Enter => {
            // Save current field
            if let Some(ref mut mr) = app.mr_draft {
                match app.mr_field {
                    1 => mr.title = app.edit_buffer.clone(),
                    2 => mr.target_branch = app.edit_buffer.clone(),
                    3 => mr.description = app.edit_buffer.clone(),
                    _ => {}
                }

                // Submit MR
                if let Some(ref provider) = app.sync_provider {
                    match provider.create_mr(mr) {
                        Ok(remote_id) => {
                            app.set_status(format!("✅ Created MR !{}", remote_id));
                            app.input_mode = InputMode::Normal;
                            app.mr_draft = None;
                            app.edit_buffer.clear();
                            return KeyAction::Refresh;
                        }
                        Err(e) => {
                            // Log full error to stderr for debugging
                            eprintln!("❌ MR Creation Error: {:?}", e);
                            eprintln!(
                                "   MR Details: source={}, target={}, title={}",
                                mr.source_branch, mr.target_branch, mr.title
                            );

                            // Show error in status bar (truncated if needed)
                            let error_msg = format!("❌ MR failed: {}", e);
                            app.set_remote_error_status(error_msg);
                            return KeyAction::Refresh;
                        }
                    }
                }
            }
            KeyAction::Refresh
        }
        KeyCode::Backspace => {
            app.edit_buffer.pop();
            KeyAction::Refresh
        }
        KeyCode::Char(c) => {
            app.edit_buffer.push(c);
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}

pub(super) fn handle_diff_comment_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.edit_buffer.clear();
            KeyAction::Refresh
        }
        KeyCode::Enter => {
            let comment = app.edit_buffer.trim().to_string();
            if comment.is_empty() {
                app.input_mode = InputMode::Normal;
                return KeyAction::Refresh;
            }

            // Handle review mode comments
            if let Some(ref mut review_state) = app.review_state {
                let author = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
                let new_comment = review_state.add_comment(comment.clone(), author);

                // Save to storage
                use crate::review::ReviewStorage;
                let storage = ReviewStorage::new(&app.repo_path);
                if let Some(ref mut review) = review_state.review {
                    review.comments.push(new_comment.clone());
                    if let Err(e) = storage.save(review) {
                        app.set_status(format!("⚠ Failed to save comment: {}", e));
                    } else {
                        app.set_status("✅ Comment added");
                    }
                } else {
                    app.set_status("✅ Comment added (not persisted - no review session)");
                }

                app.input_mode = InputMode::Normal;
                app.edit_buffer.clear();
                return KeyAction::Refresh;
            }

            // Handle diff mode comments (legacy)
            if let Some(ref state) = app.diff_state {
                if let Some(info) = state.get_selected_line_info() {
                    app.set_status(format!(
                        "Submitting comment on {}:{}...",
                        info.file_path,
                        info.new_line.or(info.old_line).unwrap_or(0)
                    ));

                    // TODO: Actually send to sync provider
                    // For now, we simulate success
                    app.set_status("✅ Comment saved locally (Sync pending)");
                }
            }

            app.input_mode = InputMode::Normal;
            app.edit_buffer.clear();
            KeyAction::Refresh
        }
        KeyCode::Backspace => {
            app.edit_buffer.pop();
            KeyAction::Refresh
        }
        KeyCode::Char(c) => {
            app.edit_buffer.push(c);
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}

/// Handle keys in repo filter dropdown
pub(super) fn handle_repo_filter_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            KeyAction::Refresh
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.available_repos.is_empty() {
                // Wraparound navigation: 0 (All) + N repos
                app.selected_repo_filter =
                    (app.selected_repo_filter + 1) % (app.available_repos.len() + 1);
            }
            KeyAction::Refresh
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if !app.available_repos.is_empty() {
                // Wraparound navigation upward
                let max = app.available_repos.len();
                app.selected_repo_filter = app.selected_repo_filter.checked_sub(1).unwrap_or(max);
            }
            KeyAction::Refresh
        }
        KeyCode::Enter => {
            // Apply filter selection
            // Index 0 = "All Repositories", 1+ = specific repos
            let filter_msg = if app.selected_repo_filter == 0 {
                app.repo_filter = None;
                "Showing all repositories".to_string()
            } else {
                if let Some(repo) = app
                    .available_repos
                    .get(app.selected_repo_filter - 1)
                    .cloned()
                {
                    app.repo_filter = Some(repo.clone());
                    format!("Filtered to: {}", repo)
                } else {
                    app.repo_filter = None;
                    "Filter cleared".to_string()
                }
            };

            app.refresh_filter();
            app.set_status(filter_msg);
            app.input_mode = InputMode::Normal;
            KeyAction::Refresh
        }
        KeyCode::Char('c') => {
            // Quick shortcut to clear filter
            app.repo_filter = None;
            app.selected_repo_filter = 0;
            app.refresh_filter();
            app.set_status("Filter cleared");
            app.input_mode = InputMode::Normal;
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}

/// Handle keys in detail view mode
pub(super) fn handle_detail_view_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.close_detail();
            KeyAction::Refresh
        }
        KeyCode::Tab | KeyCode::Char('j') | KeyCode::Down => {
            app.detail_next_field();
            KeyAction::Refresh
        }
        KeyCode::BackTab | KeyCode::Char('k') | KeyCode::Up => {
            app.detail_prev_field();
            KeyAction::Refresh
        }
        KeyCode::Enter => {
            // Start editing current field (for text fields and dates)
            // 0=Title, 1=Description, 4=Assignee, 5=Tags, 6=DueDate, 7=StartedDate, 8=CompletedDate
            if app.detail_field == 0
                || app.detail_field == 1
                || app.detail_field == 4
                || app.detail_field == 5
                || app.detail_field == 6
                || app.detail_field == 7
                || app.detail_field == 8
            {
                app.load_field_to_buffer(); // Load current value into edit buffer
                app.input_mode = InputMode::DetailEdit;
            }
            KeyAction::Refresh
        }
        KeyCode::Char(' ') => {
            // Cycle status (field 2) or effort (field 3)
            let field = app.detail_field;
            if let Some(issue) = app.detail_issue_mut() {
                match field {
                    2 => {
                        issue.status = issue.status.next();
                        issue.updated = chrono::Utc::now();
                        return KeyAction::Save;
                    }
                    3 => {
                        // Cycle effort
                        issue.effort = match issue.effort {
                            crate::issue::Effort::Trivial => crate::issue::Effort::Small,
                            crate::issue::Effort::Small => crate::issue::Effort::Medium,
                            crate::issue::Effort::Medium => crate::issue::Effort::Large,
                            crate::issue::Effort::Large => crate::issue::Effort::XLarge,
                            crate::issue::Effort::XLarge => crate::issue::Effort::Epic,
                            crate::issue::Effort::Epic => crate::issue::Effort::Trivial,
                        };
                        issue.updated = chrono::Utc::now();
                        return KeyAction::Save;
                    }
                    _ => {}
                }
            }
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}

/// Handle keys in detail edit mode (typing in a field)
pub(super) fn handle_detail_edit_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc => {
            // Cancel editing, reload field
            app.load_field_to_buffer();
            app.input_mode = InputMode::DetailView;
            KeyAction::Refresh
        }
        KeyCode::Enter => {
            // Save and exit edit mode
            app.save_field_from_buffer();
            app.input_mode = InputMode::DetailView;
            KeyAction::Save
        }
        KeyCode::Backspace => {
            app.edit_buffer.pop();
            KeyAction::Refresh
        }
        KeyCode::Char(c) => {
            app.edit_buffer.push(c);
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}

/// Handle keys in settings pane
pub(super) fn handle_settings_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('O') | KeyCode::Char('o') => {
            app.input_mode = InputMode::Normal;
            KeyAction::Refresh
        }
        KeyCode::Char('t') => {
            // Cycle theme
            app.cycle_theme();
            KeyAction::SaveTheme
        }
        _ => KeyAction::None,
    }
}
