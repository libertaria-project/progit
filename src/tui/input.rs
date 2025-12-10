//! Input - Keyboard and mouse handling
//!
//! Vim-style navigation with mode-aware key processing and mouse support.

use super::app::{App, InputMode, ViewMode};
use super::widget_kanban::{column_at_point, column_status, point_in_rect, KanbanAreas};
use super::UIAreas;
use crate::issue::Status;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

/// Result of handling an input event
#[derive(Debug, Clone)]
pub enum KeyAction {
    /// No action needed
    None,
    /// Refresh the display
    Refresh,
    /// Save current state
    Save,
    /// Save theme preference
    SaveTheme,
    /// Create new issue (optionally in specific column)
    CreateIssue(Option<Status>),
    /// Switch branch
    SwitchBranch(String),
    /// Create new branch (auto-named - deprecated)
    CreateBranch,
    /// Create new branch with specific name
    CreateBranchNamed(String),
    /// Delete a branch
    DeleteBranch(String),
    /// Delete selected issue
    DeleteIssue,
    /// Trigger sync
    Sync,
    /// Quit the application
    Quit,
}

/// Handle a key event in normal mode
pub fn handle_normal_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match app.view_mode {
        ViewMode::List => handle_list_key(app, key),
        ViewMode::Kanban => handle_kanban_key(app, key),
    }
}

/// Handle keys in list view
fn handle_list_key(app: &mut App, key: KeyEvent) -> KeyAction {
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
        KeyCode::Char('n') => KeyAction::CreateIssue(None),
        KeyCode::Char('S') => KeyAction::Sync,
        KeyCode::Char('d') => {
            app.input_mode = InputMode::Confirm;
            app.set_status("Delete issue? (y/n)");
            KeyAction::Refresh
        }

        // View toggles
        KeyCode::Tab => {
            app.toggle_view();
            KeyAction::Refresh
        }
        KeyCode::Char('t') => {
            app.cycle_theme();
            KeyAction::SaveTheme
        }

        // Search
        KeyCode::Char('/') => {
            app.input_mode = InputMode::Search;
            app.search_query.clear();
            KeyAction::Refresh
        }

        // Command Palette
        KeyCode::Char(':') => {
            app.input_mode = InputMode::Command;
            app.command_input.clear();
            KeyAction::Refresh
        }

        // Git remotes dropdown
        KeyCode::Char('r') => {
            if app.repo_info.is_some() {
                app.input_mode = InputMode::RemoteDropdown;
                KeyAction::Refresh
            } else {
                KeyAction::None
            }
        }

        // Git branch dropdown
        KeyCode::Char('b') => {
            if app.repo_info.is_some() {
                app.input_mode = InputMode::BranchDropdown;
                KeyAction::Refresh
            } else {
                KeyAction::None
            }
        }

        // Create Merge Request
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
                let target_branch = crate::git::repository::suggest_target_branch(
                    std::path::Path::new(&repo.path)
                ).unwrap_or_else(|_| "main".to_string());
                
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

        // Quit
        KeyCode::Char('q') => KeyAction::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => KeyAction::Quit,

        _ => KeyAction::None,
    }
}

/// Handle keys in kanban view
fn handle_kanban_key(app: &mut App, key: KeyEvent) -> KeyAction {
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

        // Delete
        KeyCode::Char('d') => {
            app.input_mode = InputMode::Confirm;
            app.set_status("Delete issue? (y/n)");
            KeyAction::Refresh
        }

        // View toggles
        KeyCode::Tab => {
            app.toggle_view();
            KeyAction::Refresh
        }
        KeyCode::Char('t') => {
            app.cycle_theme();
            KeyAction::SaveTheme
        }

        // Git dropdown
        KeyCode::Char('r') => {
            if app.repo_info.is_some() {
                app.input_mode = InputMode::RemoteDropdown;
                KeyAction::Refresh
            } else {
                KeyAction::None
            }
        }

        // Git branch dropdown
        KeyCode::Char('b') => {
            if app.repo_info.is_some() {
                app.input_mode = InputMode::BranchDropdown;
                KeyAction::Refresh
            } else {
                KeyAction::None
            }
        }

        // Quit
        KeyCode::Char('q') => KeyAction::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => KeyAction::Quit,

        _ => KeyAction::None,
    }
}

/// Handle a key event in search mode
pub fn handle_search_key(app: &mut App, key: KeyEvent) -> KeyAction {
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
pub fn handle_confirm_key(app: &mut App, key: KeyEvent) -> KeyAction {
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
fn handle_command_key(app: &mut App, key: KeyEvent) -> KeyAction {
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
pub fn handle_remote_dropdown_key(app: &mut App, key: KeyEvent) -> KeyAction {
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

pub fn handle_branch_dropdown_key(app: &mut App, key: KeyEvent) -> KeyAction {
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
        KeyCode::Char('d') => {
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
pub fn handle_branch_delete_confirm_key(app: &mut App, key: KeyEvent) -> KeyAction {
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
pub fn handle_branch_create_key(app: &mut App, key: KeyEvent) -> KeyAction {
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

/// Main key event handler - dispatches based on input mode
pub fn handle_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match app.input_mode {
        InputMode::Normal => handle_normal_key(app, key),
        InputMode::Search => handle_search_key(app, key),
        InputMode::Confirm => handle_confirm_key(app, key),
        InputMode::RemoteDropdown => handle_remote_dropdown_key(app, key),
        InputMode::BranchDropdown => handle_branch_dropdown_key(app, key),
        InputMode::BranchCreate => handle_branch_create_key(app, key),
        InputMode::BranchDeleteConfirm => handle_branch_delete_confirm_key(app, key),
        InputMode::DetailView => handle_detail_view_key(app, key),
        InputMode::DetailEdit => handle_detail_edit_key(app, key),
        InputMode::Command => handle_command_key(app, key),
        InputMode::MRCreate => handle_mr_create_key(app, key),
        InputMode::Edit => {
            // Legacy - redirect to detail view
            if key.code == KeyCode::Esc {
                app.input_mode = InputMode::Normal;
            }
            KeyAction::Refresh
        }
    }
}

/// Handle keys in MR creation form
fn handle_mr_create_key(app: &mut App, key: KeyEvent) -> KeyAction {
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
                            eprintln!("   MR Details: source={}, target={}, title={}", 
                                mr.source_branch, mr.target_branch, mr.title);
                            
                            // Show error in status bar (truncated if needed)
                            let error_msg = format!("❌ MR failed: {}", e);
                            app.set_status(error_msg);
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

/// Handle keys in detail view mode
fn handle_detail_view_key(app: &mut App, key: KeyEvent) -> KeyAction {
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
fn handle_detail_edit_key(app: &mut App, key: KeyEvent) -> KeyAction {
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

/// Handle mouse events
pub fn handle_mouse(app: &mut App, mouse: MouseEvent, ui_areas: &UIAreas) -> KeyAction {
    // Get current time for double-click detection
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // Check if clicking detail pane close button [X]
            if let Some(close_btn) = ui_areas.detail_close_btn {
                if point_in_rect(mouse.column, mouse.row, close_btn) {
                    app.close_detail();
                    return KeyAction::Refresh;
                }
            }

            // If detail pane is open, handle clicks
            if let Some(detail_area) = ui_areas.detail_pane {
                if !point_in_rect(mouse.column, mouse.row, detail_area) {
                    // Click outside - close detail pane
                    app.close_detail();
                    return KeyAction::Refresh;
                }
                
                // Click inside detail pane - cycle to next field
                // (Simple approach: any click in detail advances field)
                // More sophisticated field detection could be added later
                if app.input_mode == InputMode::DetailView {
                    // Check if clicking in bottom half (date fields) vs top half
                    let detail_center_y = detail_area.y + (detail_area.height / 2);
                    if mouse.row > detail_center_y {
                        // Bottom half - jump to date fields (field 6-8)
                        if app.detail_field < 6 {
                            app.detail_field = 6;
                        } else {
                            app.detail_field = (app.detail_field + 1) % 9;
                            if app.detail_field < 6 {
                                app.detail_field = 6;
                            }
                        }
                    } else {
                        // Top half - cycle through main fields (0-5)
                        app.detail_field = (app.detail_field + 1) % 6;
                    }
                    app.load_field_to_buffer();
                    return KeyAction::Refresh;
                }
                return KeyAction::None;
            }

            // Click on git branch
            if point_in_rect(mouse.column, mouse.row, ui_areas.git_branch) {
                if app.repo_info.is_some() {
                    app.input_mode = InputMode::BranchDropdown;
                    return KeyAction::Refresh;
                }
            }

            // Click on git remote
            if point_in_rect(mouse.column, mouse.row, ui_areas.git_remote) {
                if app.repo_info.is_some() {
                    app.input_mode = InputMode::RemoteDropdown;
                    return KeyAction::Refresh;
                }
                return KeyAction::None;
            }

            // Ignore clicks in status bar
            if point_in_rect(mouse.column, mouse.row, ui_areas.status_bar) {
                return KeyAction::None;
            }

            // Only handle clicks in content area
            if !point_in_rect(mouse.column, mouse.row, ui_areas.content) {
                return KeyAction::None;
            }

            // Handle kanban view
            if app.view_mode == ViewMode::Kanban {
                if let Some(col) = column_at_point(mouse.column, mouse.row, &ui_areas.kanban) {
                    let issues = app.issues_for_column(col);
                    // Offset for window frame border + column header
                    let row_offset = mouse.row.saturating_sub(ui_areas.kanban.columns[col].y + 2);

                    if let Some(&issue) = issues.get(row_offset as usize) {
                        let issue_id = issue.id.clone();

                        // Check for double-click (within 400ms on same issue)
                        let is_double_click = app.last_click_issue.as_ref() == Some(&issue_id)
                            && now.saturating_sub(app.last_click_time) < 400;

                        if is_double_click {
                            // Open detail pane
                            app.open_detail(&issue_id);
                            app.last_click_issue = None;
                            return KeyAction::Refresh;
                        } else {
                            // First click - start drag and record for double-click
                            app.drag_state.dragging_issue = Some(issue_id.clone());
                            app.drag_state.start_column = Some(col);
                            app.kanban_column = col;
                            app.kanban_row = row_offset as usize;
                            app.last_click_time = now;
                            app.last_click_issue = Some(issue_id);
                            return KeyAction::Refresh;
                        }
                    }
                }
            }

            // Handle list view
            if app.view_mode == ViewMode::List {
                if let Some(issue) = app.selected_issue() {
                    let issue_id = issue.id.clone();

                    let is_double_click = app.last_click_issue.as_ref() == Some(&issue_id)
                        && now.saturating_sub(app.last_click_time) < 400;

                    if is_double_click {
                        app.open_detail(&issue_id);
                        app.last_click_issue = None;
                        return KeyAction::Refresh;
                    } else {
                        app.last_click_time = now;
                        app.last_click_issue = Some(issue_id);
                    }
                }
            }

            KeyAction::None
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            // Update hover column during drag
            if app.drag_state.dragging_issue.is_some() {
                app.drag_state.hover_column = column_at_point(mouse.column, mouse.row, &ui_areas.kanban);
                return KeyAction::Refresh;
            }
            KeyAction::None
        }
        MouseEventKind::Up(MouseButton::Left) => {
            // Complete drag - move issue to new column
            if let Some(ref issue_id) = app.drag_state.dragging_issue.clone() {
                if let Some(target_col) = app.drag_state.hover_column {
                    let new_status = column_status(target_col);
                    if app.move_issue_to_status(issue_id, new_status) {
                        app.set_status(format!("Moved to {}", new_status.as_str()));
                        app.drag_state = Default::default();
                        return KeyAction::Save;
                    }
                }
                app.drag_state = Default::default();
                return KeyAction::Refresh;
            }
            KeyAction::None
        }
        MouseEventKind::ScrollDown => {
            match app.view_mode {
                ViewMode::List => app.next(),
                ViewMode::Kanban => app.kanban_down(),
            }
            KeyAction::Refresh
        }
        MouseEventKind::ScrollUp => {
            match app.view_mode {
                ViewMode::List => app.previous(),
                ViewMode::Kanban => app.kanban_up(),
            }
            KeyAction::Refresh
        }
        _ => KeyAction::None,
    }
}

/// Get help text for current mode
pub fn help_text(app: &App) -> &'static str {
    match app.input_mode {
        InputMode::Normal => match app.view_mode {
            ViewMode::List => "j/k:nav │ Space:status │ n:new │ M:MR │ S:sync │ d:del │ Tab:kanban │ /:search │ q:quit",
            ViewMode::Kanban => "hjkl:nav │ Enter:details │ H/L:move │ n:new │ M:MR │ S:sync │ Space:status │ Tab:list │ q:quit",
        },
        InputMode::Search => "Type to search │ Enter:confirm │ Esc:cancel",
        InputMode::Confirm => "y:yes │ n:no │ Esc:cancel",
        InputMode::RemoteDropdown => "j/k:nav │ Enter:select │ Esc:cancel",
        InputMode::BranchDropdown => "j/k:nav │ Enter:select │ n:new │ d:delete │ Esc:cancel",
        InputMode::BranchCreate => "Type branch name │ Enter:create │ Esc:cancel",
        InputMode::BranchDeleteConfirm => "y:confirm delete │ n/Esc:cancel",
        InputMode::Edit => "Esc:done │ Enter:save",
        InputMode::DetailView => "j/k:fields │ Space:cycle │ Enter:edit │ Esc:close",
        InputMode::DetailEdit => "Type to edit │ Enter:save │ Esc:cancel",
        InputMode::Command => "Type command │ Enter:exec │ Esc:cancel",
        InputMode::MRCreate => "Tab:next field │ Enter:submit │ Esc:cancel",
    }
}
