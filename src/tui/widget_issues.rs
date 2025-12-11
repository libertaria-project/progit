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
    let engine = &app.theme_engine;

    // Table header
    let header_style = engine.get("issues.header", colors.header());
    let header_cells = ["ID", "Title", "Status", "Effort", "Repo", "Tags"]
        .iter()
        .map(|h| Cell::from(*h).style(header_style));
    let header = Row::new(header_cells).height(1);

    // Table rows
    let rows: Vec<Row> = app
        .filtered
        .iter()
        .filter_map(|&idx| app.issues.get(idx))
        .map(|issue| issue_row(issue, &colors, engine))
        .collect();

    // Calculate dynamic column widths based on content
    // ID: min 8, max 15 based on actual IDs
    let max_id_len = app.filtered.iter()
        .filter_map(|&idx| app.issues.get(idx))
        .map(|i| i.short_id().len())
        .max()
        .unwrap_or(8)
        .clamp(8, 15) as u16;
    
    // Repo: min 4, max 12 based on actual repo names
    let max_repo_len = app.filtered.iter()
        .filter_map(|&idx| app.issues.get(idx))
        .filter_map(|i| i.repo.as_ref())
        .map(|r| r.len())
        .max()
        .unwrap_or(4)
        .clamp(4, 12) as u16;

    // Column widths - dynamic where sensible
    let widths = [
        Constraint::Length(max_id_len + 2),  // ID (dynamic)
        Constraint::Percentage(35),          // Title (flex)
        Constraint::Length(12),              // Status (fixed)
        Constraint::Length(6),               // Effort (fixed)
        Constraint::Length(max_repo_len + 2),// Repo (dynamic)
        Constraint::Fill(1),                 // Tags (fill remaining)
    ];

    // Create table
    let border_style = engine.get("issues.border", colors.border());
    let selected_style = engine.get("issues.row.selected", colors.selected());
    
    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(table_title(app))
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .row_highlight_style(selected_style)
        .highlight_symbol("▶ ");

    // Render with selection state
    let mut state = TableState::default();
    state.select(Some(app.selected));

    frame.render_stateful_widget(table, area, &mut state);
}

/// Create a row for an issue
fn issue_row<'a>(issue: &'a Issue, colors: &ThemeColors, engine: &crate::tui::style::ThemeEngine) -> Row<'a> {
    // 1. Determine urgency first
    let is_blocker = issue.is_blocker();
    let is_overdue = issue.is_overdue();
    
    // 2. Define the Base Style for the Row
    // Blocker = RED (Critical) - Use Theme RGB
    // Overdue = YELLOW/ORANGE (Warning) - Use Theme RGB
    let base_style = if is_blocker {
        // Use the explicit error color from the theme (RGB)
        // This avoids any ANSI color mapping issues in the terminal
        colors.error().add_modifier(Modifier::BOLD)
    } else if is_overdue {
        // Use the explicit warning color from the theme (RGB)
        colors.warning()
    } else {
        engine.get("issues.row.normal", colors.normal())
    };

    // 3. Define specific usage styles (only used if NOT urgent/overdue)
    let status_color = match issue.status {
        Status::Backlog => colors.dim(),         // Grey
        Status::InProgress => colors.success(),  // Green
        Status::Done => Style::default().fg(ratatui::style::Color::Cyan), // Cyan
    };

    // 4. Icons
    let status_icon = match issue.status {
        Status::Backlog => "○",
        Status::InProgress => "▶",
        Status::Done => "✔",
    };

    let blocker_indicator = if is_blocker { 
        "🔥 " 
    } else if is_overdue { 
        "⏰ " 
    } else { 
        "" 
    };
    
    // 5. Repo Display
    let repo_display = issue.repo.as_deref().unwrap_or("-");
    let repo_style = if issue.repo.is_some() && !is_blocker && !is_overdue {
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
    } else if is_blocker || is_overdue {
        base_style // Inherit Red or Yellow
    } else {
        colors.dim()
    };

    // 6. Tags
    let mut tag_spans = Vec::new();
    for (i, tag) in issue.tags.iter().enumerate() {
        if i > 0 {
            tag_spans.push(Span::raw(" "));
        }
        let tag_style = if is_blocker || is_overdue { base_style } else { colors.accent() };
        tag_spans.push(Span::styled(format!("[ {} ]", tag), tag_style));
    }
    let tags_cell = if tag_spans.is_empty() {
        Cell::from("")
    } else {
        Cell::from(Line::from(tag_spans))
    };

    // 7. Construct Cells
    let is_urgent = is_blocker || is_overdue;
    
    let cells = [
        // ID: Dim unless urgent
        Cell::from(issue.short_id().to_string()).style(if is_urgent { base_style } else { colors.dim() }),
        
        // Title: ALWAYS base_style (Red if urgent, Normal otherwise)
        Cell::from(format!("{}{}", blocker_indicator, &issue.title)).style(base_style),
        
        // Status: Inherit if urgent, else Status Color
        Cell::from(format!("{} {}", status_icon, issue.status.as_str()))
            .style(if is_urgent { base_style } else { status_color }),
            
        // Effort: base_style
        Cell::from(format!("{}", issue.effort as u8)).style(base_style),
        
        // Repo: Inherit if urgent, else Rainbow/Dim
        Cell::from(repo_display).style(repo_style),
        
        // Tags
        tags_cell.style(base_style),
    ];

    Row::new(cells).style(base_style)
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
