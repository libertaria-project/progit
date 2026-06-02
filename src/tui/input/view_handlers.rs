//! View-mode key handlers (non-modal views)

use super::super::app::{App, InputMode, ViewMode};
use super::KeyAction;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle keys in MR list view
pub(super) fn handle_mr_list_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.mr_list.is_empty() {
                app.mr_selected = (app.mr_selected + 1) % app.mr_list.len();
            }
            KeyAction::Refresh
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if !app.mr_list.is_empty() {
                app.mr_selected = app
                    .mr_selected
                    .checked_sub(1)
                    .unwrap_or(app.mr_list.len() - 1);
            }
            KeyAction::Refresh
        }
        KeyCode::Char('g') => {
            app.mr_selected = 0;
            KeyAction::Refresh
        }
        KeyCode::Char('G') => {
            if !app.mr_list.is_empty() {
                app.mr_selected = app.mr_list.len() - 1;
            }
            KeyAction::Refresh
        }
        KeyCode::Enter => {
            if let Some(mr) = app.mr_list.get(app.mr_selected) {
                // Determine diff reference: target...source
                // NOTE: For local, this works. For remote, we might need to fetch first?
                // The DiffState logic handles `git diff arguments`.
                let diff_ref = format!("{}...{}", mr.target_branch, mr.source_branch);
                app.set_status(format!("Loading diff for {}...", mr.source_branch));

                let mut state = crate::diff::DiffState::new_with_mode(
                    crate::diff::DiffMode::Custom(diff_ref.clone()),
                );
                match state.load(&app.repo_path) {
                    Ok(_) => {
                        app.diff_state = Some(state);
                        app.set_status(format!("Diff: {}", diff_ref));
                        app.view_mode = ViewMode::Diff;
                        KeyAction::Refresh
                    }
                    Err(e) => {
                        app.set_status(format!("Diff failed: {}", e));
                        KeyAction::Refresh
                    }
                }
            } else {
                KeyAction::None
            }
        }
        KeyCode::Char('r') => {
            app.set_status("Reloading MRs...");
            if let Err(e) = app.refresh_mrs() {
                app.set_remote_error_status(format!("Failed: {}", e));
            }
            KeyAction::Refresh
        }

        KeyCode::Char('?') => {
            use crate::tui::input::help_text;
            app.set_status(help_text(app));
            KeyAction::Refresh
        }

        // MR-specific actions
        KeyCode::Char('a') => {
            // Approve MR
            if let Some(mr) = app.mr_list.get(app.mr_selected).cloned() {
                if let Some(remote_id) = mr.remote_id {
                    if let Some(ref provider) = app.sync_provider {
                        match provider.approve_mr(remote_id) {
                            Ok(_) => {
                                app.set_status(format!(
                                    "👍 Review approved for MR !{} (LGTM)",
                                    remote_id
                                ));
                                // Reload MRs to reflect changes
                                let _ = app.refresh_mrs();
                            }
                            Err(e) => {
                                app.set_remote_error_status(format!("❌ Failed to approve: {}", e));
                            }
                        }
                    } else {
                        app.set_status("No sync provider configured".to_string());
                    }
                } else {
                    app.set_status("MR has no remote ID".to_string());
                }
            }
            KeyAction::Refresh
        }
        KeyCode::Char('m') => {
            // Merge MR
            if let Some(mr) = app.mr_list.get(app.mr_selected).cloned() {
                if let Some(remote_id) = mr.remote_id {
                    if let Some(ref provider) = app.sync_provider {
                        match provider.merge_mr(remote_id) {
                            Ok(_) => {
                                app.set_status(format!("✅ Accepted & Merged MR !{}", remote_id));
                                // Reload MRs to reflect changes
                                let _ = app.refresh_mrs();
                            }
                            Err(e) => {
                                app.set_remote_error_status(format!("❌ Failed to merge: {}", e));
                            }
                        }
                    } else {
                        app.set_status("No sync provider configured".to_string());
                    }
                } else {
                    app.set_status("MR has no remote ID".to_string());
                }
            }
            KeyAction::Refresh
        }
        KeyCode::Char('x') => {
            // Reject MR (close without merging)
            if let Some(mr) = app.mr_list.get(app.mr_selected).cloned() {
                if let Some(remote_id) = mr.remote_id {
                    if let Some(ref provider) = app.sync_provider {
                        match provider.close_mr(remote_id) {
                            Ok(_) => {
                                app.set_status(format!(
                                    "❌ Rejected MR !{} (closed without merge)",
                                    remote_id
                                ));
                                // Reload MRs to reflect changes
                                let _ = app.refresh_mrs();
                            }
                            Err(e) => {
                                app.set_remote_error_status(format!("❌ Failed to reject: {}", e));
                            }
                        }
                    } else {
                        app.set_status("No sync provider configured".to_string());
                    }
                } else {
                    app.set_status("MR has no remote ID".to_string());
                }
            }
            KeyAction::Refresh
        }

        // Command Palette
        KeyCode::Char(':') => {
            app.input_mode = InputMode::Command;
            app.command_input.clear();
            KeyAction::Refresh
        }

        KeyCode::Tab => {
            app.toggle_view();
            KeyAction::Refresh
        }
        KeyCode::Char('q') => {
            // Go back to Kanban or List?
            app.view_mode = ViewMode::Kanban;
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}

/// Handle keys in diff view
pub(super) fn handle_diff_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        // Tab: Toggle between Staged and Unstaged
        KeyCode::Tab => {
            if let Some(ref mut state) = app.diff_state {
                // Toggle mode
                state.mode = match state.mode {
                    crate::diff::DiffMode::Unstaged => crate::diff::DiffMode::Staged,
                    crate::diff::DiffMode::Staged => crate::diff::DiffMode::Unstaged,
                    crate::diff::DiffMode::Custom(_) => crate::diff::DiffMode::Unstaged,
                };
                state.scroll = 0;
                state.cursor_y = 0;

                // Reload diff with new mode
                if let Some(ref info) = app.repo_info {
                    let _ = state.load(std::path::Path::new(&info.path));
                }
            }
            KeyAction::Refresh
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            app.view_mode = ViewMode::List;
            app.diff_state = None;
            KeyAction::Refresh
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(ref mut state) = app.diff_state {
                let total = state.total_visible_lines();
                if state.cursor_y < total.saturating_sub(1) {
                    state.cursor_y += 1;
                    // Handle scrolling
                    if state.cursor_y >= state.scroll as usize + 20 {
                        state.scroll = (state.cursor_y.saturating_sub(19)) as u16;
                    }
                }
            }
            KeyAction::Refresh
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Some(ref mut state) = app.diff_state {
                if state.cursor_y > 0 {
                    state.cursor_y -= 1;
                    // Handle scrolling
                    if state.cursor_y < state.scroll as usize {
                        state.scroll = state.cursor_y as u16;
                    }
                }
            }
            KeyAction::Refresh
        }
        KeyCode::Char('J') => {
            if let Some(ref mut state) = app.diff_state {
                if state.selected_file < state.files.len().saturating_sub(1) {
                    state.selected_file += 1;
                    state.scroll = 0;
                    state.cursor_y = 0;
                }
            }
            KeyAction::Refresh
        }
        KeyCode::Char('K') => {
            if let Some(ref mut state) = app.diff_state {
                if state.selected_file > 0 {
                    state.selected_file -= 1;
                    state.scroll = 0;
                    state.cursor_y = 0;
                }
            }
            KeyAction::Refresh
        }
        // Collapsing
        KeyCode::Char(' ') => {
            if let Some(ref mut state) = app.diff_state {
                if let Some(file) = state.files.get_mut(state.selected_file) {
                    file.collapsed = !file.collapsed;
                }
                state.clamp_cursor();
            }
            KeyAction::Refresh
        }
        KeyCode::Char('h') => {
            if let Some(ref mut state) = app.diff_state {
                if let Some(file) = state.files.get_mut(state.selected_file) {
                    // Toggle all hunks
                    let all_collapsed = file.hunks.iter().all(|h| h.collapsed);
                    for hunk in &mut file.hunks {
                        hunk.collapsed = !all_collapsed;
                    }
                }
                state.clamp_cursor();
            }
            KeyAction::Refresh
        }
        // Commenting
        KeyCode::Char('c') => {
            app.input_mode = InputMode::DiffComment;
            app.edit_buffer.clear();
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}

/// Handle keys in blame view
pub(super) fn handle_blame_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.view_mode = ViewMode::List;
            app.blame_state = None;
            KeyAction::Refresh
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(ref mut state) = app.blame_state {
                if let Some(info) = &state.info {
                    let max_lines = info.lines.len();
                    let current = state.table_state.selected().unwrap_or(0);
                    if current < max_lines.saturating_sub(1) {
                        state.table_state.select(Some(current + 1));
                    }
                }
            }
            KeyAction::Refresh
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Some(ref mut state) = app.blame_state {
                let current = state.table_state.selected().unwrap_or(0);
                if current > 0 {
                    state.table_state.select(Some(current - 1));
                }
            }
            KeyAction::Refresh
        }
        KeyCode::Char('g') => {
            if let Some(ref mut state) = app.blame_state {
                state.table_state.select(Some(0));
            }
            KeyAction::Refresh
        }
        KeyCode::Char('G') => {
            if let Some(ref mut state) = app.blame_state {
                if let Some(info) = &state.info {
                    state
                        .table_state
                        .select(Some(info.lines.len().saturating_sub(1)));
                }
            }
            KeyAction::Refresh
        }
        KeyCode::Char('m') => {
            // Toggle Mode
            if let Some(ref mut state) = app.blame_state {
                use crate::tui::widget_blame::BlameMode;
                state.mode = match state.mode {
                    BlameMode::Manager => BlameMode::LeadDev,
                    BlameMode::LeadDev => BlameMode::Manager,
                };
            }
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}

/// Handle keys in lanes view (virtual branches)
pub(super) fn handle_lanes_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.view_mode = ViewMode::List;
            KeyAction::Refresh
        }
        // Navigate between lanes (left/right)
        KeyCode::Char('h') | KeyCode::Left => {
            app.vbranch_selected = app.vbranch_selected.saturating_sub(1);
            KeyAction::Refresh
        }
        KeyCode::Char('l') | KeyCode::Right => {
            if let Some(ref manager) = app.vbranch_manager {
                let max = manager.list().len().saturating_sub(1);
                app.vbranch_selected = (app.vbranch_selected + 1).min(max);
            }
            KeyAction::Refresh
        }
        // Navigate within lane (up/down for hunks)
        KeyCode::Char('j') | KeyCode::Down => {
            app.vbranch_hunk_selected = app.vbranch_hunk_selected.saturating_add(1);
            KeyAction::Refresh
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.vbranch_hunk_selected = app.vbranch_hunk_selected.saturating_sub(1);
            KeyAction::Refresh
        }
        // Create new virtual branch
        KeyCode::Char('n') => {
            app.edit_buffer = String::from("feature/");
            app.input_mode = InputMode::VBranchCreate;
            app.set_status("Enter virtual branch name...");
            KeyAction::Refresh
        }
        // Toggle staging for selected hunk
        KeyCode::Char(' ') | KeyCode::Enter => {
            if let Some(ref mut manager) = app.vbranch_manager {
                let branches = manager.list();
                if let Some(branch) = branches.get(app.vbranch_selected) {
                    let branch_id = branch.id.clone();
                    let owned_hunks = branch.owned_hunks.clone();
                    let staged_hunks = branch.staged_hunks.clone();

                    // Get selected hunk
                    if let Some(hunk) = owned_hunks.get(app.vbranch_hunk_selected) {
                        // Check if hunk is already staged
                        let is_staged = staged_hunks.contains(hunk);

                        if let Some(branch) = manager.get_mut(&branch_id) {
                            if is_staged {
                                branch.unstage_hunk(hunk);
                                app.set_status("Hunk unstaged");
                            } else {
                                branch.stage_hunk(hunk);
                                app.set_status("Hunk staged");
                            }
                        }
                    } else {
                        app.set_status("No hunk selected");
                    }
                }
            }
            KeyAction::Refresh
        }
        // Transfer hunk to another lane
        KeyCode::Char('m') => {
            if let Some(ref manager) = app.vbranch_manager {
                let branches = manager.list();
                if branches.len() < 2 {
                    app.set_status("Need at least 2 branches to move hunks");
                } else if let Some(branch) = branches.get(app.vbranch_selected) {
                    if branch.owned_hunks.get(app.vbranch_hunk_selected).is_some() {
                        app.input_mode = InputMode::VBranchMove;
                        app.set_status("Select target lane (h/l), Enter to confirm, Esc to cancel");
                    } else {
                        app.set_status("No hunk selected to move");
                    }
                }
            }
            KeyAction::Refresh
        }
        // Open AI Agent Menu
        KeyCode::Char('a') => {
            // Check if we have a selected branch with hunks
            if let Some(manager) = &app.vbranch_manager {
                if let Some(_branch) = manager.list().get(app.vbranch_selected) {
                    app.show_agent_menu = true;
                    app.agent_menu_selected = 0; // Reset selection
                    app.set_status("Select an AI action...");
                } else {
                    app.set_status("No branch selected");
                }
            } else {
                app.set_status("Virtual branches not initialized");
            }
            KeyAction::Refresh
        }
        // Show conflict resolution modal
        KeyCode::Char('c') => {
            if let Some(ref manager) = app.vbranch_manager {
                let conflicts = manager.detect_conflicts();
                if conflicts.is_empty() {
                    app.set_status("No conflicts detected");
                } else {
                    app.show_conflicts = true;
                    app.set_status(format!("{} branch(es) have conflicts", conflicts.len()));
                }
            }
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}

/// Handle keys in review mode
pub(super) fn handle_review_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        // Navigation
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(state) = &mut app.review_state {
                state.move_down(30); // Visible lines estimate
            }
            KeyAction::Refresh
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Some(state) = &mut app.review_state {
                state.move_up();
            }
            KeyAction::Refresh
        }

        // Add comment
        KeyCode::Char('c') => {
            app.input_mode = InputMode::DiffComment;
            app.edit_buffer.clear();
            KeyAction::Refresh
        }

        // Sync local review comments to the configured forge.
        //
        // [DEBT] v1 stub: surfaces the CLI command that does the work.
        // Full in-TUI push (MR-id resolution from local UUID → remote_id,
        // SyncProvider lifecycle inside the render loop, async progress
        // indicator) is Sprint D scope.
        KeyCode::Char('S') => {
            app.set_status(
                "Push review comments via CLI: `prog mr review push <mr_id>`",
            );
            KeyAction::Refresh
        }

        // Quit review mode
        KeyCode::Char('q') | KeyCode::Esc => {
            app.review_state = None;
            app.view_mode = ViewMode::Dashboard;
            KeyAction::Refresh
        }

        _ => KeyAction::None,
    }
}

/// Handle keys in list view
pub(super) fn handle_list_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        // Navigation
        KeyCode::Char('j') | KeyCode::Down => {
            app.next();
            KeyAction::Refresh
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.previous();
            KeyAction::Refresh
        }
        KeyCode::Char('g') => {
            app.selected = 0;
            KeyAction::Refresh
        }
        KeyCode::Char('G') => {
            if !app.filtered.is_empty() {
                app.selected = app.filtered.len() - 1;
            }
            KeyAction::Refresh
        }

        // Actions
        KeyCode::Char(' ') => {
            app.cycle_selected_status();
            KeyAction::Save
        }
        KeyCode::Enter => {
            // Open detail pane for selected issue
            if let Some(issue) = app.selected_issue() {
                let id = issue.id.clone();
                app.open_detail(&id);
            }
            KeyAction::Refresh
        }
        KeyCode::Char('d') => {
            // Quick Diff: Show dirty changes (working tree vs index)
            app.set_status("Loading local changes...");
            let mut state = crate::diff::DiffState::new_with_mode(crate::diff::DiffMode::Unstaged);
            match state.load(&app.repo_path) {
                Ok(_) => {
                    app.diff_state = Some(state);
                    app.view_mode = ViewMode::Diff;
                    KeyAction::Refresh
                }
                Err(e) => {
                    app.set_status(format!("No local changes: {}", e));
                    KeyAction::Refresh
                }
            }
        }
        KeyCode::Char('n') => KeyAction::CreateIssue(None),
        KeyCode::Char('M') => {
            // Check for git repo first
            if app.repo_info.is_none() {
                app.set_status("⚠️  MR creation requires a git repository");
                return KeyAction::Refresh;
            }

            // Check for sync provider
            if app.sync_provider.is_none() {
                app.set_status("⚠️  No sync provider configured. Run 'prog sync' first or add sync config to .project/config.kdl");
                return KeyAction::Refresh;
            }

            // Initialize MR draft with smart defaults
            if let Some(ref repo) = app.repo_info {
                let source_branch = repo.branch.clone();
                let target_branch =
                    crate::git::repository::suggest_target_branch(std::path::Path::new(&repo.path))
                        .unwrap_or_else(|_| "main".to_string());

                // Auto-generate title from branch name
                let title = source_branch
                    .replace("feature/", "")
                    .replace("bugfix/", "Fix: ")
                    .replace("hotfix/", "Hotfix: ")
                    .replace('-', " ")
                    .replace('_', " ");

                app.mr_draft = Some(crate::mr::MergeRequest::new(
                    &source_branch,
                    &target_branch,
                    &title,
                ));
                app.mr_field = 1; // Start on title field (0=source is readonly)
                app.edit_buffer = title;
                app.input_mode = InputMode::MRCreate;
            }
            KeyAction::Refresh
        }

        // Fuzzy Command Palette (Ctrl+P)
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.input_mode = InputMode::FuzzyPalette;
            app.fuzzy_query.clear();
            app.fuzzy_selected = 0;
            KeyAction::Refresh
        }

        // Quit
        KeyCode::Char('q') => KeyAction::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => KeyAction::Quit,

        _ => KeyAction::None,
    }
}

/// Handle keys in kanban view
pub(super) fn handle_kanban_key(app: &mut App, key: KeyEvent) -> KeyAction {
    use super::super::widget_kanban::column_status;

    match key.code {
        // Navigation
        KeyCode::Char('j') | KeyCode::Down => {
            app.kanban_down();
            KeyAction::Refresh
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.kanban_up();
            KeyAction::Refresh
        }
        KeyCode::Char('h') | KeyCode::Left => {
            app.kanban_left();
            KeyAction::Refresh
        }
        KeyCode::Char('l') | KeyCode::Right => {
            app.kanban_right();
            KeyAction::Refresh
        }

        // Move issue between columns (Shift + h/l)
        KeyCode::Char('H') => {
            if app.kanban_move_left() {
                KeyAction::Save
            } else {
                KeyAction::Refresh
            }
        }
        KeyCode::Char('L') => {
            if app.kanban_move_right() {
                KeyAction::Save
            } else {
                KeyAction::Refresh
            }
        }

        // Space cycles status of selected issue (Move Right)
        KeyCode::Char(' ') => {
            if let Some(issue) = app.kanban_selected_issue() {
                let id = issue.id.clone();
                if let Some(issue) = app.issue_by_id_mut(&id) {
                    // Shift+Space for Previous (Move Left)
                    if key.modifiers.contains(KeyModifiers::SHIFT) {
                        issue.status = issue.status.prev();
                    } else {
                        issue.status = issue.status.next();
                    }
                    return KeyAction::Save;
                }
            }
            KeyAction::Refresh
        }

        // Backspace for Previous (Move Left)
        KeyCode::Backspace => {
            if let Some(issue) = app.kanban_selected_issue() {
                let id = issue.id.clone();
                if let Some(issue) = app.issue_by_id_mut(&id) {
                    issue.status = issue.status.prev();
                    return KeyAction::Save;
                }
            }
            KeyAction::Refresh
        }

        // New issue in current column
        KeyCode::Char('n') => {
            let status = column_status(app.kanban_column);
            KeyAction::CreateIssue(Some(status))
        }

        // Open detail view
        KeyCode::Enter => {
            if let Some(issue) = app.kanban_selected_issue() {
                let id = issue.id.clone();
                app.open_detail(&id);
                KeyAction::Refresh
            } else {
                KeyAction::None
            }
        }
        KeyCode::Char('d') => {
            // Quick Diff: Show dirty changes
            app.set_status("Loading local changes...");
            let mut state = crate::diff::DiffState::new_with_mode(crate::diff::DiffMode::Unstaged);
            match state.load(&app.repo_path) {
                Ok(_) => {
                    app.diff_state = Some(state);
                    app.view_mode = ViewMode::Diff;
                    KeyAction::Refresh
                }
                Err(e) => {
                    app.set_status(format!("No local changes: {}", e));
                    KeyAction::Refresh
                }
            }
        }

        // Delete
        KeyCode::Char('D') => {
            app.input_mode = InputMode::Confirm;
            app.set_status("Delete issue? (y/n)");
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}
