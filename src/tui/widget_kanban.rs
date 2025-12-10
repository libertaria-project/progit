//! Widget Kanban - Kanban board view with drag-drop support
//!
//! 3-column board: Backlog | In Progress | Done
//! With + buttons at top/bottom for quick issue creation.

use crate::issue::{Issue, Status};
use crate::tui::app::App;
use crate::tui::theme::ThemeColors;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

/// Kanban column areas for mouse hit detection
#[derive(Debug, Clone, Default)]
pub struct KanbanAreas {
    pub columns: [Rect; 3],
    pub add_buttons: [Rect; 3], // Top + buttons
}

/// Render the kanban board
pub fn render(frame: &mut Frame, area: Rect, app: &App) -> KanbanAreas {
    let colors = app.theme.colors();
    let mut areas = KanbanAreas::default();

    // Split into 3 columns
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    areas.columns = [columns[0], columns[1], columns[2]];

    let (backlog, in_progress, done) = app.issues_by_status();

    // Render each column
    render_column(
        frame,
        columns[0],
        "📥 Backlog",
        &backlog,
        &colors,
        Status::Backlog,
        app.kanban_column == 0,
        if app.kanban_column == 0 { Some(app.kanban_row) } else { None },
        app.drag_state.hover_column == Some(0),
    );
    render_column(
        frame,
        columns[1],
        "🔄 In Progress",
        &in_progress,
        &colors,
        Status::InProgress,
        app.kanban_column == 1,
        if app.kanban_column == 1 { Some(app.kanban_row) } else { None },
        app.drag_state.hover_column == Some(1),
    );
    render_column(
        frame,
        columns[2],
        "✅ Done",
        &done,
        &colors,
        Status::Done,
        app.kanban_column == 2,
        if app.kanban_column == 2 { Some(app.kanban_row) } else { None },
        app.drag_state.hover_column == Some(2),
    );

    areas
}

/// Render a single kanban column
#[allow(clippy::too_many_arguments)]
fn render_column(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    issues: &[&Issue],
    colors: &ThemeColors,
    status: Status,
    is_selected_column: bool,
    selected_row: Option<usize>,
    is_drag_target: bool,
) {
    let status_style = match status {
        Status::Backlog => colors.dim(),
        Status::InProgress => colors.success(),
        Status::Done => colors.accent(),
    };

    // Split area for + button at top, list, + button at bottom
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top + button
            Constraint::Min(3),    // Issues list
            Constraint::Length(1), // Bottom + button
        ])
        .split(area);

    // Top + button
    let top_btn_style = if is_selected_column {
        colors.accent()
    } else {
        colors.dim()
    };
    let top_btn = Paragraph::new(Line::from(vec![
        Span::styled("  [+] New Issue", top_btn_style),
    ]));
    frame.render_widget(top_btn, chunks[0]);

    // Create list items
    let items: Vec<ListItem> = issues
        .iter()
        .enumerate()
        .map(|(idx, issue)| {
            let is_selected = selected_row == Some(idx);
            let _is_dragging = matches!(&issue.id, id if Some(id.as_str()) == None); 

            let prefix = if is_selected { "▶ " } else { "  " };
            let blocker_mark = if issue.is_blocker() { "🔥" } else if issue.is_overdue() { "⏰" } else { "  " };

            let style = if is_selected {
                colors.selected()
            } else if issue.is_blocker() || issue.is_overdue() {
                colors.error_bg()
            } else if issue.status == Status::InProgress {
                 colors.success_bg()
            } else if issue.status == Status::Done {
                 colors.done_bg()
            } else {
                colors.normal()
            };

            let content = format!(
                "{}{}{} [{}]",
                prefix,
                blocker_mark,
                truncate_title(&issue.title, 20),
                issue.effort as u8
            );

            ListItem::new(content).style(style)
        })
        .collect();

    // Calculate column stats
    let total_effort: u32 = issues.iter().map(|i| i.effort as u32).sum();

    // Column border style
    let border_style = if is_drag_target {
        colors.accent().add_modifier(Modifier::BOLD)
    } else if is_selected_column {
        status_style
    } else {
        colors.border()
    };

    // Create column block
    let block = Block::default()
        .title(Line::from(vec![
            Span::raw(title),
            Span::raw(" "),
            Span::styled(
                format!("({} pts)", total_effort),
                Style::default().add_modifier(Modifier::DIM),
            ),
        ]))
        .borders(Borders::ALL)
        .border_style(border_style);

    let list = List::new(items).block(block);

    frame.render_widget(list, chunks[1]);

    // Bottom + button (for quick add at bottom of column)
    let bottom_btn = Paragraph::new(Line::from(vec![
        Span::styled("  [+]", colors.dim()),
    ]));
    frame.render_widget(bottom_btn, chunks[2]);
}

/// Truncate title to fit column
fn truncate_title(title: &str, max_len: usize) -> String {
    if title.len() <= max_len {
        title.to_string()
    } else {
        format!("{}…", &title[..max_len - 1])
    }
}

/// Get the column index for a status
pub fn status_column(status: Status) -> usize {
    match status {
        Status::Backlog => 0,
        Status::InProgress => 1,
        Status::Done => 2,
    }
}

/// Get the status for a column index
pub fn column_status(column: usize) -> Status {
    match column {
        0 => Status::Backlog,
        1 => Status::InProgress,
        _ => Status::Done,
    }
}

/// Check if point is within a rect
pub fn point_in_rect(x: u16, y: u16, rect: Rect) -> bool {
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}

/// Find which column a point is in
pub fn column_at_point(x: u16, y: u16, areas: &KanbanAreas) -> Option<usize> {
    for (i, col) in areas.columns.iter().enumerate() {
        if point_in_rect(x, y, *col) {
            return Some(i);
        }
    }
    None
}
