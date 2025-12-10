//! TUI Feature - Sovereign Index
//!
//! Terminal user interface with Ratatui + Crossterm.
//! All TUI logic lives in `tui/` folder.
//!
//! ARCHITECTURE: Tiling window manager style
//! - All views are wrapped in titled frames (windows)
//! - Git bar at top, status bar at bottom
//! - Main content area contains the active view window

pub mod app;
pub mod input;
pub mod theme;
pub mod widget_detail;
pub mod widget_issues;
pub mod widget_kanban;
pub mod widget_mr_create;
pub mod widget_status;

// Re-export public API
pub use app::{App, DragState, InputMode, ViewMode};
pub use input::{handle_key, handle_mouse, help_text, KeyAction};
pub use theme::{Theme, ThemeColors};
pub use widget_detail::EditField;
pub use widget_kanban::KanbanAreas;

use crate::git::{render_gitbar, render_remote_dropdown};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, BorderType},
    Frame,
};

/// UI areas for mouse hit detection
#[derive(Debug, Clone, Default)]
pub struct UIAreas {
    /// Git bar area (not clickable for issues)
    pub git_bar: Rect,
    /// Git Branch area
    pub git_branch: Rect,
    /// Git Remote area
    pub git_remote: Rect,
    /// Main content area
    pub content: Rect,
    /// Status bar area
    pub status_bar: Rect,
    /// Kanban column areas
    pub kanban: KanbanAreas,
    /// Detail pane area (if visible)
    pub detail_pane: Option<Rect>,
    /// Detail pane close button [X]
    pub detail_close_btn: Option<Rect>,
}

/// Render the entire UI, returns UI areas for mouse handling
pub fn render(frame: &mut Frame, app: &mut App) -> UIAreas {
    let size = frame.area();
    let colors = app.theme.colors();
    let mut areas = UIAreas::default();

    // Main layout: git bar + content area + status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // Git bar
            Constraint::Min(3),     // Content
            Constraint::Length(2),  // Status bar
        ])
        .split(size);


    areas.git_bar = chunks[0];
    areas.content = chunks[1];
    areas.status_bar = chunks[2];

    // PO/PM Metrics bar at top
    use crate::git::widget_gitbar::render_with_app as render_pm_bar;
    let (branch_area, remote_area) = render_pm_bar(frame, chunks[0], app, &colors);
    areas.git_branch = branch_area;
    areas.git_remote = remote_area;


    // Render view based on mode - wrapped in titled frame
    areas.kanban = match app.view_mode {
        ViewMode::List => {
            // Calculate repo stats
            let repo_stats = calculate_repo_stats(&app.issues);
            
            // Create titled frame for list view
            let mut title_spans = vec![
                Span::styled(" 📋 Issues ", colors.accent()),
                Span::styled(format!("({} total) ", app.issues.len()), colors.dim()),
            ];
            
            // Add repo stats if multi-repo
            if !repo_stats.is_empty() {
                title_spans.push(Span::raw("│ "));
                title_spans.push(Span::styled("📦 ", colors.dim()));
                for (i, (repo, count)) in repo_stats.iter().enumerate() {
                    if i > 0 {
                        title_spans.push(Span::raw(" │ "));
                    }
                    title_spans.push(Span::styled(
                        format!("{}: {}", repo, count),
                        colors.accent()
                    ));
                }
            }
            
            let block = Block::default()
                .title(Line::from(title_spans))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(colors.border());

            let inner = block.inner(chunks[1]);
            frame.render_widget(block, chunks[1]);
            widget_issues::render(frame, inner, app);
            KanbanAreas::default()
        }
        ViewMode::Kanban => {
            // Calculate repo stats
            let repo_stats = calculate_repo_stats(&app.issues);
            
            // Create titled frame for kanban view
            let mut title_spans = vec![
                Span::styled(" 📊 Kanban ", colors.accent()),
                Span::styled(format!("({} issues) ", app.issues.len()), colors.dim()),
            ];
            
            // Add repo stats if multi-repo
            if !repo_stats.is_empty() {
                title_spans.push(Span::raw("│ "));
                title_spans.push(Span::styled("📦 ", colors.dim()));
                for (i, (repo, count)) in repo_stats.iter().enumerate() {
                    if i > 0 {
                        title_spans.push(Span::raw(" │ "));
                    }
                    title_spans.push(Span::styled(
                        format!("{}: {}", repo, count),
                        colors.accent()
                    ));
                }
            }
            
            let block = Block::default()
                .title(Line::from(title_spans))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(colors.border());

            let inner = block.inner(chunks[1]);
            frame.render_widget(block, chunks[1]);
            widget_kanban::render(frame, inner, app)
        }
    };

    // Status bar at bottom - now with &mut App
    widget_status::render(frame, chunks[2], app);

    // Render dropdown overlay if open
    if app.input_mode == InputMode::RemoteDropdown {
        if let Some(ref repo) = app.repo_info {
            // Position dropdown below git bar
            let dropdown_height = (repo.remotes.len() + 2) as u16;
            let dropdown_area = Rect {
                x: chunks[0].x + 20, // Offset from left
                y: chunks[0].y + chunks[0].height,
                width: 50.min(size.width - 20),
                height: dropdown_height.min(10),
            };
            render_remote_dropdown(frame, dropdown_area, repo, app.selected_remote, &colors);
        }
    }

    // Render branch dropdown overlay if open
    if app.input_mode == InputMode::BranchDropdown {
        if let Some(ref repo) = app.repo_info {
             // Position dropdown below branch area
             // Height = branches + 1 (new branch option) + 2 (border)
             let dropdown_height = (repo.branches.len() + 3) as u16;
             let dropdown_area = Rect {
                 x: chunks[0].x + 5, // Offset
                 y: chunks[0].y + chunks[0].height,
                 width: 35,
                 height: dropdown_height.min(15),
             };
             use crate::git::render_branch_dropdown;
             render_branch_dropdown(frame, dropdown_area, repo, app.selected_branch, &colors);
        }
    }

    // Render branch name input overlay
    if app.input_mode == InputMode::BranchCreate {
        let input_area = Rect {
            x: chunks[0].x + 5,
            y: chunks[0].y + chunks[0].height,
            width: 40,
            height: 3,
        };
        use crate::git::render_branch_input;
        render_branch_input(frame, input_area, &app.edit_buffer, &colors);
    }

    // Render detail pane overlay if open
    if matches!(app.input_mode, InputMode::DetailView | InputMode::DetailEdit) {
        if let Some(issue) = app.detail_issue() {
            let edit_field = EditField::from_index(app.detail_field);
            let (detail_area, close_btn) = widget_detail::render(frame, issue, edit_field, &app.edit_buffer, &colors);
            areas.detail_pane = Some(detail_area);
            areas.detail_close_btn = Some(close_btn);
        }
    }

    // Render MR creation form overlay if open
    if app.input_mode == InputMode::MRCreate {
        if let Some(ref mr) = app.mr_draft {
            widget_mr_create::render(frame, mr, app.mr_field, &app.edit_buffer, &colors);
        }
    }

    areas
}

/// Calculate repository statistics from issues
fn calculate_repo_stats(issues: &[crate::issue::Issue]) -> Vec<(String, usize)> {
    use std::collections::HashMap;
    
    let mut repo_counts: HashMap<String, usize> = HashMap::new();
    
    for issue in issues {
        if let Some(ref repo) = issue.repo {
            *repo_counts.entry(repo.clone()).or_insert(0) += 1;
        }
    }
    
    // Sort by count (descending) for consistent display
    let mut stats: Vec<(String, usize)> = repo_counts.into_iter().collect();
    stats.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        let app = App::new();
        assert_eq!(app.view_mode, ViewMode::List);
        assert_eq!(app.input_mode, InputMode::Normal);
    }
}
