//! Widget Issues - Issue table widget
//!
//! Sortable table view of all issues.

use crate::issue::{Issue, Status};
use crate::tui::app::App;
use crate::tui::theme::ThemeColors;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

/// Render the issues table
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let colors = app.theme.colors();

    // Table header
    let header_cells = ["ID", "Title", "Status", "Effort", "Repo", "Tags"]
        .iter()
        .map(|h| Cell::from(*h).style(colors.header()));
    let header = Row::new(header_cells).height(1);

    // Table rows
    let rows: Vec<Row> = app
        .filtered
        .iter()
        .filter_map(|&idx| app.issues.get(idx))
        .map(|issue| issue_row(issue, &colors))
        .collect();

    // Column widths
    let widths = [
        Constraint::Length(10),     // ID
        Constraint::Percentage(35), // Title (reduced to make room)
        Constraint::Length(12),     // Status
        Constraint::Length(8),      // Effort
        Constraint::Length(12),     // Repo
        Constraint::Percentage(20), // Tags
    ];

    // Create table
    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(table_title(app))
                .borders(Borders::ALL)
                .border_style(colors.border()),
        )
        .row_highlight_style(colors.selected())
        .highlight_symbol("▶ ");

    // Render with selection state
    let mut state = TableState::default();
    state.select(Some(app.selected));

    frame.render_stateful_widget(table, area, &mut state);
}

/// Create a row for an issue
fn issue_row<'a>(issue: &'a Issue, colors: &ThemeColors) -> Row<'a> {
    // Determine row style based on status and blocking state
    let row_style = if issue.is_blocker() || issue.is_overdue() {
        colors.error_bg()
    } else if issue.status == Status::InProgress {
        colors.success_bg()
    } else if issue.status == Status::Done {
        colors.done_bg()
    } else {
        colors.normal()
    };

    let status_style = match issue.status {
        Status::Backlog => colors.dim(),
        Status::InProgress => colors.success(),
        Status::Done => colors.accent(),
    };

    let blocker_indicator = if issue.is_blocker() { 
        "🔥 " 
    } else if issue.is_overdue() { 
        "⏰ " 
    } else { 
        "" 
    };
    
    // Repo display with color coding
    let repo_display = issue.repo.as_deref().unwrap_or("-");
    let repo_style = if issue.repo.is_some() {
        // Color-code by repo name (simple hash-based coloring)
        let hash = repo_display.bytes().fold(0u8, |acc, b| acc.wrapping_add(b));
        let color = match hash % 6 {
            0 => ratatui::style::Color::Cyan,
            1 => ratatui::style::Color::Magenta,
            2 => ratatui::style::Color::Yellow,
            3 => ratatui::style::Color::Green,
            4 => ratatui::style::Color::Blue,
            _ => ratatui::style::Color::Red,
        };
        Style::default().fg(color)
    } else {
        colors.dim()
    };

    let cells = [
        Cell::from(issue.short_id().to_string()),
        Cell::from(format!("{}{}", blocker_indicator, &issue.title)),
        Cell::from(issue.status.as_str()).style(status_style),
        Cell::from(format!("{}", issue.effort as u8)),
        Cell::from(repo_display).style(repo_style),
        Cell::from(issue.tags.join(", ")),
    ];

    Row::new(cells).style(row_style)
}

/// Generate table title with stats
fn table_title(app: &App) -> Line<'static> {
    let total = app.issues.len();
    let done = app.issues.iter().filter(|i| i.status == Status::Done).count();
    let blockers = app.blocker_count();

    let mut spans = vec![
        Span::raw(" Issues "),
        Span::styled(
            format!("[{}/{}]", done, total),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ];

    if blockers > 0 {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            format!("🔥{} blockers", blockers),
            Style::default().fg(ratatui::style::Color::Red),
        ));
    }

    Line::from(spans)
}
