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
pub mod agent_executor;
pub mod input;
pub mod style;
pub mod theme;
pub mod widget_agent_menu;
pub mod widget_blame;
pub mod widget_conflicts;
pub mod widget_dashboard;
mod widget_debug;
pub mod widget_detail;
mod widget_fuzzy_palette;
pub mod widget_issues;
pub mod widget_kanban;
pub mod widget_lanes;
pub mod widget_mr_create;
pub mod widget_mr_list;
pub mod widget_pano_log;
pub mod widget_settings;
pub mod widget_status;

// Re-export public API
pub use app::{App, DragState, InputMode, ViewMode};
pub use input::{handle_key, handle_mouse, help_text, KeyAction};
pub use theme::{Theme, ThemeColors};
pub use widget_detail::EditField;
pub use widget_kanban::KanbanAreas;

use crate::git::render_remote_dropdown;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
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
    /// Help icon area in status bar
    pub help_icon: Option<Rect>,
    /// Dashboard Tab
    pub tab_dashboard: Option<Rect>,
    /// Tab: Issues (clickable)
    pub tab_issues: Option<Rect>,
    /// Tab: Kanban (clickable)
    pub tab_kanban: Option<Rect>,
    /// Tab: MRs
    pub tab_mrs: Option<Rect>,
    /// Tab: Settings (clickable, right-aligned)
    pub tab_settings: Option<Rect>,
    /// Diff view file list area (left pane)
    pub diff_file_list: Option<Rect>,
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
            Constraint::Length(2), // Git bar
            Constraint::Min(3),    // Content
            Constraint::Length(2), // Status bar
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

    // Render view based on mode - with clickable tab title bar
    // Create the tabbed title: [🏠 Dash] [📋 Issues] [📊 Kanban] [🔀 MRs] ... [⚙ Settings]
    let dash_label = " 🏠 Dash ";
    let issues_label = " 📋 Issues ";
    let kanban_label = " 📊 Kanban ";
    let mrs_label = " 🔀 MRs ";
    let settings_label = " ⚙ Settings ";

    // Calculate tab widths for click detection
    let tab_start_x = areas.content.x + 1; // After border
    let dash_tab_width = dash_label.chars().count() as u16 + 2;
    let issues_tab_width = issues_label.chars().count() as u16 + 2;
    let kanban_tab_width = kanban_label.chars().count() as u16 + 2;
    let mrs_tab_width = mrs_label.chars().count() as u16 + 2;
    let settings_tab_width = settings_label.chars().count() as u16 + 2;

    // Store tab areas for mouse hit detection
    areas.tab_dashboard = Some(Rect {
        x: tab_start_x,
        y: areas.content.y,
        width: dash_tab_width,
        height: 1,
    });
    areas.tab_issues = Some(Rect {
        x: tab_start_x + dash_tab_width + 1,
        y: areas.content.y,
        width: issues_tab_width,
        height: 1,
    });
    areas.tab_kanban = Some(Rect {
        x: tab_start_x + dash_tab_width + issues_tab_width + 2,
        y: areas.content.y,
        width: kanban_tab_width,
        height: 1,
    });
    areas.tab_mrs = Some(Rect {
        x: tab_start_x + dash_tab_width + issues_tab_width + kanban_tab_width + 3,
        y: areas.content.y,
        width: mrs_tab_width,
        height: 1,
    });

    // Settings tab (right-aligned)
    let settings_x = areas.content.x + areas.content.width - settings_tab_width - 2;
    areas.tab_settings = Some(Rect {
        x: settings_x,
        y: areas.content.y,
        width: settings_tab_width,
        height: 1,
    });

    // Build title spans with active/inactive styling
    let (dash_style, issues_style, kanban_style, mr_style) = match app.view_mode {
        ViewMode::Dashboard => (
            colors
                .accent()
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            colors.dim(),
            colors.dim(),
            colors.dim(),
        ),
        ViewMode::List => (
            colors.dim(),
            colors
                .accent()
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            colors.dim(),
            colors.dim(),
        ),
        ViewMode::Kanban => (
            colors.dim(),
            colors.dim(),
            colors
                .accent()
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            colors.dim(),
        ),
        ViewMode::MRList => (
            colors.dim(),
            colors.dim(),
            colors.dim(),
            colors
                .accent()
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        ),
        ViewMode::Diff => (colors.dim(), colors.dim(), colors.dim(), colors.dim()),
        ViewMode::Blame => (colors.dim(), colors.dim(), colors.dim(), colors.dim()),
        ViewMode::Lanes => (colors.dim(), colors.dim(), colors.dim(), colors.dim()),
    };

    // Settings style (highlighted if in settings mode)
    let settings_style = if app.input_mode == InputMode::Settings {
        colors
            .accent()
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        colors.dim()
    };

    // Calculate padding for right-alignment
    let left_content_len = dash_label.len()
        + 1
        + issues_label.len()
        + 1
        + kanban_label.len()
        + 1
        + mrs_label.len()
        + format!(" ({} total) ", app.issues.len()).len();
    let total_width = areas.content.width as usize - 2; // Minus borders
    let padding = if total_width > left_content_len + settings_label.len() {
        total_width - left_content_len - settings_label.len()
    } else {
        1
    };

    let title_spans = vec![
        Span::styled(dash_label, dash_style),
        Span::raw("│"),
        Span::styled(issues_label, issues_style),
        Span::raw("│"),
        Span::styled(kanban_label, kanban_style),
        Span::raw("│"),
        Span::styled(mrs_label, mr_style),
        Span::styled(format!(" ({} issues) ", app.issues.len()), colors.dim()),
        Span::raw(" ".repeat(padding)),
        Span::styled(settings_label, settings_style),
    ];

    let block = Block::default()
        .title(Line::from(title_spans))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(colors.border());

    let inner = block.inner(chunks[1]);
    frame.render_widget(block, chunks[1]);

    // Render the actual view content
    areas.kanban = match app.view_mode {
        ViewMode::Dashboard => {
            widget_dashboard::render(frame, inner, app);
            KanbanAreas::default()
        }
        ViewMode::List => {
            widget_issues::render(frame, inner, app);
            KanbanAreas::default()
        }
        ViewMode::Kanban => widget_kanban::render(frame, inner, app),
        ViewMode::MRList => {
            widget_mr_list::render(frame, inner, app);
            KanbanAreas::default()
        }
        ViewMode::Diff => {
            if let Some(ref state) = app.diff_state {
                areas.diff_file_list = crate::diff::render_diff(frame, inner, state);
            }
            KanbanAreas::default()
        }
        ViewMode::Blame => {
            crate::tui::widget_blame::render(frame, inner, app);
            KanbanAreas::default()
        }
        ViewMode::Lanes => {
            widget_lanes::render(frame, inner, app);
            KanbanAreas::default()
        }
    };

    // Status bar at bottom - now with &mut App
    areas.help_icon = widget_status::render(frame, chunks[2], app);

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
            render_remote_dropdown(
                frame,
                dropdown_area,
                repo,
                app.selected_remote,
                &colors,
                &app.theme_engine,
            );
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
            render_branch_dropdown(
                frame,
                dropdown_area,
                repo,
                app.selected_branch,
                &colors,
                &app.theme_engine,
            );
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
        render_branch_input(
            frame,
            input_area,
            &app.edit_buffer,
            &colors,
            &app.theme_engine,
        );
    }

    // Render repository filter dropdown overlay if open
    if app.input_mode == InputMode::RepoFilter {
        if !app.available_repos.is_empty() {
            // Position dropdown below git bar
            // Height = repos + 1 ("All") + 2 (border)
            let dropdown_height = (app.available_repos.len() + 3) as u16;
            let dropdown_area = Rect {
                x: chunks[0].x + 5,
                y: chunks[0].y + chunks[0].height,
                width: 40,
                height: dropdown_height.min(15),
            };
            render_repo_filter_dropdown(frame, dropdown_area, app, &colors);
        }
    }

    // Render detail pane overlay if open
    if matches!(
        app.input_mode,
        InputMode::DetailView | InputMode::DetailEdit
    ) {
        if let Some(issue) = app.detail_issue() {
            let edit_field = widget_detail::EditField::from_index(app.detail_field);
            let (detail_area, close_btn) =
                widget_detail::render(frame, issue, edit_field, &app.edit_buffer, &colors);
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

    // Render debug console overlay if enabled
    if app.show_debug_console {
        let area = centered_rect(size, 80, 50);
        // Clear background for overlay
        frame.render_widget(ratatui::widgets::Clear, area);
        widget_debug::render(frame, area, app);
    }

    // Render settings pane overlay if open
    if app.input_mode == InputMode::Settings {
        widget_settings::render(frame, app, 0);
    }

    // Render fuzzy command palette overlay if open (Ctrl+P)
    if app.input_mode == InputMode::FuzzyPalette {
        widget_fuzzy_palette::render(frame, app, &colors);
    }

    // Render Comment Input Box if open
    if app.input_mode == InputMode::DiffComment {
        let area = centered_rect(size, 60, 20);
        frame.render_widget(ratatui::widgets::Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Add Code Comment (Esc to cancel, Enter to save) ")
            .border_style(colors.accent());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Show current line info summary
        let mut lines = Vec::new();
        if let Some(ref state) = app.diff_state {
            if let Some(info) = state.get_selected_line_info() {
                lines.push(Line::from(vec![
                    Span::styled("File: ", colors.dim()),
                    Span::styled(
                        info.file_path.clone(),
                        colors.normal().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" :{}", info.new_line.or(info.old_line).unwrap_or(0)),
                        colors.dim(),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("Text: ", colors.dim()),
                    Span::styled(
                        info.content.trim().to_string(),
                        colors.dim().add_modifier(Modifier::ITALIC),
                    ),
                ]));
                lines.push(Line::raw(""));
            }
        }

        lines.push(Line::from(Span::styled(&app.edit_buffer, colors.normal())));
        // Cursor
        lines.push(Line::from(Span::styled("█", colors.accent())));

        let p = Paragraph::new(lines).wrap(ratatui::widgets::Wrap { trim: false });
        frame.render_widget(p, inner);
    }

    // Overlays (render last so they appear on top)
    if app.show_pano_log {
        widget_pano_log::render(frame, app);
    }
    
    // Conflict resolution modal
    if app.show_conflicts {
        widget_conflicts::render(frame, app);
    }
    
    // Agent menu modal (top layer)
    if app.show_agent_menu {
        widget_agent_menu::render(frame, app, app.agent_menu_selected);
    }

    areas
}

/// Helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(r: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
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

/// Render the repository filter dropdown menu
fn render_repo_filter_dropdown(frame: &mut Frame, area: Rect, app: &App, colors: &ThemeColors) {
    use ratatui::style::Modifier;
    use ratatui::widgets::{Clear, Paragraph};

    let engine = &app.theme_engine;

    // Clear background first to prevent bleed-through
    frame.render_widget(Clear, area);

    let mut items: Vec<Line> = Vec::new();

    // Styles
    let normal_style = engine.get("dropdown.filter.normal", colors.normal());
    let selected_style = engine.get("dropdown.filter.selected", colors.selected());
    let dim_style = engine.get("dropdown.filter.dim", colors.dim());

    // First item: "All Repositories" (index 0)
    let all_prefix = if app.selected_repo_filter == 0 {
        "▶ "
    } else {
        "  "
    };
    let all_style = if app.selected_repo_filter == 0 {
        selected_style
    } else {
        normal_style
    };

    items.push(Line::from(vec![
        Span::styled(all_prefix, all_style),
        Span::styled("All Repositories", all_style.add_modifier(Modifier::BOLD)),
        Span::styled(format!(" ({})", app.issues.len()), dim_style),
    ]));

    // Add each repository with issue count
    for (i, repo) in app.available_repos.iter().enumerate() {
        let idx = i + 1; // Offset by 1 since 0 is "All"
        let prefix = if app.selected_repo_filter == idx {
            "▶ "
        } else {
            "  "
        };
        let style = if app.selected_repo_filter == idx {
            selected_style
        } else {
            normal_style
        };

        // Count issues for this repo
        let count = app
            .issues
            .iter()
            .filter(|issue| issue.repo.as_ref().map_or(false, |r| r == repo))
            .count();

        // Color-code by repo name (same logic as issue table)
        // Allow override via config: "repo.color.frontend"
        let repo_config_key = format!("repo.color.{}", repo);

        let repo_style = if app.selected_repo_filter == idx {
            style // Keep selection style if selected
        } else {
            // Check if specific repo color is configured, otherwise use hash
            let hash = repo.bytes().fold(0u8, |acc, b| acc.wrapping_add(b));
            let default_color = match hash % 6 {
                0 => ratatui::style::Color::Cyan,
                1 => ratatui::style::Color::Magenta,
                2 => ratatui::style::Color::Yellow,
                3 => ratatui::style::Color::Green,
                4 => ratatui::style::Color::Blue,
                _ => ratatui::style::Color::Red,
            };
            engine.get(
                &repo_config_key,
                ratatui::style::Style::default().fg(default_color),
            )
        };

        items.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(repo, repo_style.add_modifier(Modifier::BOLD)),
            Span::styled(format!(" ({})", count), dim_style),
        ]));
    }

    let border_style = engine.get("dropdown.border", colors.accent());
    let title_style = engine.get("dropdown.title", colors.header());

    let dropdown = Paragraph::new(items).block(
        Block::default()
            .title(Span::styled(" Filter by Repository ", title_style))
            .borders(Borders::ALL)
            .border_style(border_style),
    );

    frame.render_widget(dropdown, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        let app = App::new();
        assert_eq!(app.view_mode, ViewMode::Dashboard);
        assert_eq!(app.input_mode, InputMode::Normal);
    }
}
