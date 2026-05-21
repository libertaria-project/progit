//! Input - Keyboard and mouse handling
//!
//! Vim-style navigation with mode-aware key processing and mouse support.
//! Some per-mode handlers are defined ahead of their TUI wiring — suppress until dispatched.
#![allow(dead_code)]

mod modal_handlers;
mod view_handlers;

use modal_handlers::*;
use view_handlers::*;

use super::app::{App, InputMode, ViewMode};
use super::agent_executor::execute_agent_action;
use super::widget_kanban::{column_at_point, column_status, point_in_rect};
use super::UIAreas;
use crate::issue::Status;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

/// Result of handling an input event
#[derive(Debug, Clone, PartialEq)]
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
    /// Toggle debug console
    ToggleDebug,
    /// Quit the application
    Quit,
}

/// Handle a key event in normal mode (RENAMED - old function for compat)
fn handle_normal_mode_key(app: &mut App, key: KeyEvent) -> KeyAction {
    let action = match app.view_mode {
        ViewMode::Dashboard => handle_dashboard_key(app, key),
        ViewMode::List => handle_list_key(app, key),
        ViewMode::Kanban => handle_kanban_key(app, key),
        ViewMode::Diff => handle_diff_key(app, key),
        ViewMode::MRList => handle_mr_list_key(app, key),
        ViewMode::Blame => handle_blame_key(app, key),
        ViewMode::Lanes => handle_lanes_key(app, key),
        ViewMode::Review => handle_review_key(app, key),
    };

    if action != KeyAction::None {
        return action;
    }

    // Global Fallback Keys
    match key.code {
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

        // Fuzzy Palette (Ctrl+P)
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.input_mode = InputMode::FuzzyPalette;
            app.edit_buffer.clear();
            app.fuzzy_query.clear();
            app.fuzzy_selected = 0;
            KeyAction::Refresh
        }

        // View toggles
        KeyCode::Tab => {
            app.toggle_view();
            KeyAction::Refresh
        }

        // Global Actions
        KeyCode::Char('S') => KeyAction::Sync,
        KeyCode::Char('P') | KeyCode::Char('Q') => {
            // Q is an alias for P that lands you on the plugin manager
            // modal where quarantined plugins are flagged. Same modal,
            // same close keys; one extra entry point because users
            // who hit a quarantine event think "Q" before "P".
            app.show_plugins = !app.show_plugins;
            app.plugin_selected = 0;
            KeyAction::Refresh
        }
        KeyCode::Char('q') => KeyAction::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => KeyAction::Quit,
        KeyCode::Char('O') => {
            app.input_mode = InputMode::Settings;
            KeyAction::Refresh
        }
        KeyCode::Char('t') => {
            app.cycle_theme();
            KeyAction::SaveTheme
        }
        KeyCode::Char('?') => {
            app.set_status(help_text(app));
            KeyAction::Refresh
        }

        // Git dropdowns
        KeyCode::Char('r') => {
            if app.repo_info.is_some() {
                app.input_mode = InputMode::RemoteDropdown;
                KeyAction::Refresh
            } else {
                KeyAction::None
            }
        }
        KeyCode::Char('b') => {
            if app.repo_info.is_some() {
                app.input_mode = InputMode::BranchDropdown;
                KeyAction::Refresh
            } else {
                KeyAction::None
            }
        }
        KeyCode::Char('f') => {
            if !app.available_repos.is_empty() {
                app.input_mode = InputMode::RepoFilter;
                KeyAction::Refresh
            } else {
                KeyAction::None
            }
        }
        _ => KeyAction::None,
    }
}

fn handle_dashboard_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
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
        _ => KeyAction::None,
    }
}

/// Main key event handler - dispatches based on input mode
pub fn handle_key(app: &mut App, key: KeyEvent) -> KeyAction {
    // Modal overlays take priority (close on Escape)

    // Agent menu modal
    if app.show_agent_menu {
        use crate::tui::widget_agent_menu::AgentAction;

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                app.agent_menu_selected = (app.agent_menu_selected + 1) % AgentAction::all().len();
                return KeyAction::Refresh;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if app.agent_menu_selected == 0 {
                    app.agent_menu_selected = AgentAction::all().len() - 1;
                } else {
                    app.agent_menu_selected -= 1;
                }
                return KeyAction::Refresh;
            }
            KeyCode::Enter => {
                // Execute selected action
                let action = AgentAction::all()[app.agent_menu_selected];
                app.show_agent_menu = false;

                // Trigger agent with selected action
                execute_agent_action(app, action);
                return KeyAction::Refresh;
            }
            KeyCode::Esc => {
                app.show_agent_menu = false;
                app.set_status("Agent action canceled");
                return KeyAction::Refresh;
            }
            _ => return KeyAction::None,
        }
    }

    // Conflict resolution modal
    if app.show_conflicts && key.code == KeyCode::Esc {
        app.show_conflicts = false;
        return KeyAction::Refresh;
    }

    // Panopticum log viewer
    if app.show_pano_log && key.code == KeyCode::Esc {
        app.show_pano_log = false;
        return KeyAction::Refresh;
    }

    // Plugin manager modal
    if app.show_plugins {
        match key.code {
            KeyCode::Esc
            | KeyCode::Char('P')
            | KeyCode::Char('Q')
            | KeyCode::Char('q') => {
                app.show_plugins = false;
                return KeyAction::Refresh;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let count = app.plugin_manager.as_ref().map_or(0, |pm| pm.count());
                if count > 0 {
                    app.plugin_selected = (app.plugin_selected + 1) % count;
                }
                return KeyAction::Refresh;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let count = app.plugin_manager.as_ref().map_or(0, |pm| pm.count());
                if count > 0 {
                    if app.plugin_selected == 0 {
                        app.plugin_selected = count - 1;
                    } else {
                        app.plugin_selected -= 1;
                    }
                }
                return KeyAction::Refresh;
            }
            KeyCode::Char('u') => {
                // Clear quarantine on the highlighted plugin, if any.
                if let Some(pm) = app.plugin_manager.as_mut() {
                    let infos = pm.plugin_info();
                    let idx = app.plugin_selected.min(infos.len().saturating_sub(1));
                    if let Some(meta) = infos.get(idx) {
                        let name = meta.name.clone();
                        if pm.unquarantine(&name) {
                            log::info!("Cleared quarantine on plugin '{}'", name);
                        }
                    }
                }
                return KeyAction::Refresh;
            }
            _ => return KeyAction::None,
        }
    }

    match app.input_mode {
        InputMode::Normal => handle_normal_mode_key(app, key),
        InputMode::Search => handle_search_key(app, key),
        InputMode::Confirm => handle_confirm_key(app, key),
        InputMode::RemoteDropdown => handle_remote_dropdown_key(app, key),
        InputMode::BranchDropdown => handle_branch_dropdown_key(app, key),
        InputMode::BranchCreate => handle_branch_create_key(app, key),
        InputMode::BranchDeleteConfirm => handle_branch_delete_confirm_key(app, key),
        InputMode::VBranchCreate => handle_vbranch_create_key(app, key),
        InputMode::VBranchMove => handle_vbranch_move_key(app, key),
        InputMode::DetailView => handle_detail_view_key(app, key),
        InputMode::DetailEdit => handle_detail_edit_key(app, key),
        InputMode::Command => handle_command_key(app, key),
        InputMode::MRCreate => handle_mr_create_key(app, key),
        InputMode::RepoFilter => handle_repo_filter_key(app, key),
        InputMode::Settings => handle_settings_key(app, key),
        InputMode::FuzzyPalette => handle_fuzzy_palette_key(app, key),
        InputMode::DiffComment => handle_diff_comment_key(app, key),
        InputMode::ProjectWiki => handle_project_wiki_key(app, key),
        InputMode::ProjectIssues => handle_project_issues_key(app, key),
        InputMode::Edit => {
            // Legacy - redirect to detail view
            if key.code == KeyCode::Esc {
                app.input_mode = InputMode::Normal;
            }
            KeyAction::Refresh
        }
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

            // Click on Dashboard tab
            if let Some(tab_area) = ui_areas.tab_dashboard {
                if point_in_rect(mouse.column, mouse.row, tab_area) {
                    if app.view_mode != ViewMode::Dashboard {
                        app.view_mode = ViewMode::Dashboard;
                    }
                    return KeyAction::Refresh;
                }
            }

            // Click on Issues tab
            if let Some(tab_area) = ui_areas.tab_issues {
                if point_in_rect(mouse.column, mouse.row, tab_area) {
                    if app.view_mode != ViewMode::List {
                        app.view_mode = ViewMode::List;
                    }
                    return KeyAction::Refresh;
                }
            }

            // Click on Kanban tab
            if let Some(tab_area) = ui_areas.tab_kanban {
                if point_in_rect(mouse.column, mouse.row, tab_area) {
                    if app.view_mode != ViewMode::Kanban {
                        app.view_mode = ViewMode::Kanban;
                    }
                    return KeyAction::Refresh;
                }
            }

            // Click on MRs tab
            if let Some(tab_area) = ui_areas.tab_mrs {
                if point_in_rect(mouse.column, mouse.row, tab_area) {
                    if app.view_mode != ViewMode::MRList {
                        app.view_mode = ViewMode::MRList;
                        if app.mr_list.is_empty() {
                            let _ = app.refresh_mrs();
                        }
                    }
                    return KeyAction::Refresh;
                }
            }

            // Click on Settings tab
            if let Some(tab_area) = ui_areas.tab_settings {
                if point_in_rect(mouse.column, mouse.row, tab_area) {
                    if app.input_mode == InputMode::Settings {
                        app.input_mode = InputMode::Normal; // Toggle off
                    } else {
                        app.input_mode = InputMode::Settings;
                    }
                    return KeyAction::Refresh;
                }
            }

            // Click on help icon (? help)
            if let Some(help_area) = ui_areas.help_icon {
                if point_in_rect(mouse.column, mouse.row, help_area) {
                    app.set_status(help_text(app));
                    return KeyAction::Refresh;
                }
            }

            // Click on diff file list (left pane)
            if let Some(file_list_area) = ui_areas.diff_file_list {
                if point_in_rect(mouse.column, mouse.row, file_list_area) {
                    if let Some(ref mut diff_state) = app.diff_state {
                        // Calculate clicked file index
                        // File list starts at file_list_area.y + 1 (border)
                        let list_start_y = file_list_area.y + 1;
                        if mouse.row >= list_start_y {
                            let clicked_idx = (mouse.row - list_start_y) as usize;
                            if clicked_idx < diff_state.files.len() {
                                diff_state.selected_file = clicked_idx;
                                diff_state.scroll = 0; // Reset scroll when switching files
                                return KeyAction::Refresh;
                            }
                        }
                    }
                    return KeyAction::None;
                }
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
                // Calculate which row was clicked (offset for header/border)
                // The list content starts at content.y + 3 (border + header row + separator)
                let list_start_y = ui_areas.content.y + 3;

                if mouse.row >= list_start_y {
                    let clicked_row = (mouse.row - list_start_y) as usize;

                    // Check if valid row in filtered list
                    if clicked_row < app.filtered.len() {
                        // Get the issue index and then the ID
                        let issue_idx = app.filtered[clicked_row];
                        let issue_id = app.issues[issue_idx].id.clone();

                        // Select this row
                        app.selected = clicked_row;

                        // Check for double-click
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

                        return KeyAction::Refresh;
                    }
                }
            }

            // Handle MR list view
            if app.view_mode == ViewMode::MRList {
                // Calculate which row was clicked (offset for header/border)
                // The MR list content starts at content.y + 3 (border + header row + separator)
                let list_start_y = ui_areas.content.y + 3;

                if mouse.row >= list_start_y {
                    let clicked_row = (mouse.row - list_start_y) as usize;

                    // Check if valid row in MR list
                    if clicked_row < app.mr_list.len() {
                        // Select this row
                        app.mr_selected = clicked_row;

                        // Check for double-click to open diff view
                        let mr_id = app.mr_list[clicked_row].id.to_string();
                        let is_double_click = app.last_click_issue.as_ref() == Some(&mr_id)
                            && now.saturating_sub(app.last_click_time) < 400;

                        if is_double_click {
                            // Open diff view for this MR (same as Enter key)
                            if let Some(mr) = app.mr_list.get(app.mr_selected) {
                                let diff_ref =
                                    format!("{}...{}", mr.target_branch, mr.source_branch);
                                app.set_status(format!("Loading diff for {}...", mr.source_branch));

                                let mut state = crate::diff::DiffState::new_with_mode(
                                    crate::diff::DiffMode::Custom(diff_ref),
                                );
                                match state.load(&app.repo_path) {
                                    Ok(_) => {
                                        app.diff_state = Some(state);
                                        app.view_mode = ViewMode::Diff;
                                    }
                                    Err(e) => {
                                        app.set_status(format!("Diff failed: {}", e));
                                    }
                                }
                            }
                            app.last_click_issue = None;
                            return KeyAction::Refresh;
                        } else {
                            app.last_click_time = now;
                            app.last_click_issue = Some(mr_id);
                        }

                        return KeyAction::Refresh;
                    }
                }
            }

            KeyAction::None
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            // Update hover column during drag
            if app.drag_state.dragging_issue.is_some() {
                app.drag_state.hover_column =
                    column_at_point(mouse.column, mouse.row, &ui_areas.kanban);
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
                ViewMode::Dashboard => {} // Nothing to scroll on dashboard
                ViewMode::List => app.next(),
                ViewMode::Kanban => app.kanban_down(),
                ViewMode::Diff => {
                    if let Some(ref mut state) = app.diff_state {
                        state.scroll = state.scroll.saturating_add(3);
                    }
                }
                ViewMode::MRList => {
                    if !app.mr_list.is_empty() {
                        app.mr_selected = (app.mr_selected + 1) % app.mr_list.len();
                    }
                }
                ViewMode::Blame => {
                    if let Some(ref mut state) = app.blame_state {
                        let current = state.table_state.selected().unwrap_or(0);
                        state.table_state.select(Some(current + 3));
                    }
                }
                ViewMode::Lanes => {
                    // Navigate virtual branches
                    app.vbranch_selected = app.vbranch_selected.saturating_add(1);
                }
                ViewMode::Review => {
                    if let Some(ref mut state) = app.review_state {
                        state.move_down(10); // Visible lines estimate
                    }
                }
            }
            KeyAction::Refresh
        }
        MouseEventKind::ScrollUp => {
            match app.view_mode {
                ViewMode::Dashboard => {}
                ViewMode::List => app.previous(),
                ViewMode::Kanban => app.kanban_up(),
                ViewMode::Diff => {
                    if let Some(ref mut state) = app.diff_state {
                        state.scroll = state.scroll.saturating_sub(3);
                    }
                }
                ViewMode::MRList => {
                    if !app.mr_list.is_empty() {
                        app.mr_selected = app
                            .mr_selected
                            .checked_sub(1)
                            .unwrap_or(app.mr_list.len() - 1);
                    }
                }
                ViewMode::Blame => {
                    if let Some(ref mut state) = app.blame_state {
                        let current = state.table_state.selected().unwrap_or(0);
                        if current > 3 {
                            state.table_state.select(Some(current - 3));
                        } else {
                            state.table_state.select(Some(0));
                        }
                    }
                }
                ViewMode::Lanes => {
                    // Navigate virtual branches
                    app.vbranch_selected = app.vbranch_selected.saturating_sub(1);
                }
                ViewMode::Review => {
                    if let Some(ref mut state) = app.review_state {
                        state.move_up();
                    }
                }
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
            ViewMode::Dashboard => "Tab:list │ S:sync │ P:plugins │ O:settings │ q:quit",
            ViewMode::List => "j/k:nav │ Space:status │ n:new │ M:MR │ S:sync │ d:del │ f:filter │ Tab:kanban │ /:search │ q:quit",
            ViewMode::Kanban => "hjkl:nav │ Enter:details │ H/L:move │ n:new │ M:MR │ S:sync │ f:filter │ Space:status │ Tab:list │ q:quit",
            ViewMode::Diff => "j/k:scroll │ J/K:files │ Space:collapse │ q:close",
            ViewMode::MRList => "j/k:nav │ Enter:diff │ a:approve(LGTM) │ m:accept+merge │ x:reject │ r:reload │ S:sync │ ?:help │ q:back",
            ViewMode::Blame => "j/k:scroll │ m:toggle mode │ q:back",
            ViewMode::Lanes => "h/l:lanes │ j/k:hunks │ n:new │ Space:stage │ m:move │ q:back",
            ViewMode::Review => "j/k:navigate │ c:comment │ q:back",
        },
        InputMode::Search => "Type to search │ Enter:confirm │ Esc:cancel",
        InputMode::Confirm => "y:yes │ n:no │ Esc:cancel",
        InputMode::RemoteDropdown => "j/k:nav │ Enter:select │ Esc:cancel",
        InputMode::BranchDropdown => "j/k:nav │ Enter:select │ n:new │ d:delete │ Esc:cancel",
        InputMode::BranchCreate => "Type branch name │ Enter:create │ Esc:cancel",
        InputMode::BranchDeleteConfirm => "y:confirm delete │ n/Esc:cancel",
        InputMode::VBranchCreate => "Type virtual branch name │ Enter:create │ Esc:cancel",
        InputMode::VBranchMove => "h/l:select target lane │ Enter:move hunk │ Esc:cancel",
        InputMode::Edit => "Esc:done │ Enter:save",
        InputMode::DetailView => "j/k:fields │ Space:cycle │ Enter:edit │ Esc:close",
        InputMode::DetailEdit => "Type to edit │ Enter:save │ Esc:cancel",
        InputMode::Command => "Type command │ Enter:exec │ Esc:cancel",
        InputMode::MRCreate => "Tab:next field │ Enter:submit │ Esc:cancel",
        InputMode::RepoFilter => "j/k:nav │ Enter:select │ c:clear │ Esc:cancel",
        InputMode::Settings => "t:theme │ O/Esc:close",
        InputMode::FuzzyPalette => "Type to search │ Enter:select │ Esc:close",
        InputMode::DiffComment => "Type comment │ Enter:save │ Esc:cancel",
        InputMode::ProjectWiki => "h/l:page │ j/k:scroll │ g/G:top/bottom │ q/Esc:close",
        InputMode::ProjectIssues => "j/k:nav │ Enter:open loaded issue │ q/Esc:close",
    }
}
