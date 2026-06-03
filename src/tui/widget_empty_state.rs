//! Empty State Teaching - Contextual help when views have no data
//!
//! Turns blank screens into teachable moments. Every empty state tells the user
//! exactly what to do next — no docs required.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Render a full-screen empty state for the issue list / kanban board.
/// Call this when `app.issues.is_empty()`.
pub fn render_issues_empty(frame: &mut Frame, area: Rect, colors: &crate::tui::theme::ThemeColors) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Length(14),
            Constraint::Percentage(40),
        ])
        .split(area);

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("📭  ", Style::default()),
            Span::styled("No issues yet", colors.accent().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   Press ", colors.normal()),
            Span::styled("n", colors.accent().add_modifier(Modifier::BOLD)),
            Span::styled(" to create your first issue", colors.normal()),
        ]),
        Line::from(vec![
            Span::styled("   Press ", colors.normal()),
            Span::styled("?", colors.accent().add_modifier(Modifier::BOLD)),
            Span::styled(" for the full keymap", colors.normal()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   Pro tip: ", colors.dim()),
            Span::styled("prog init --demo", colors.warning().add_modifier(Modifier::BOLD)),
            Span::styled(" seeds a sample project", colors.dim()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   ", colors.normal()),
            Span::styled("https://progit.dev", colors.dim()),
        ]),
    ];

    let paragraph = Paragraph::new(content)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(colors.border())
                .title(" Getting Started "),
        );

    frame.render_widget(paragraph, chunks[1]);
}

/// Render a compact empty-state hint inside a kanban column.
/// Call this when a single column has no issues.
pub fn render_kanban_column_empty(
    frame: &mut Frame,
    area: Rect,
    colors: &crate::tui::theme::ThemeColors,
    status_name: &str,
) {
    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("   Empty ", colors.dim()),
            Span::styled(status_name, colors.dim().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   Press ", colors.dim()),
            Span::styled("n", colors.accent().add_modifier(Modifier::BOLD)),
            Span::styled(" to add an issue", colors.dim()),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

/// Render a compact empty-state hint for the MR list.
/// Call this when `app.mr_list.is_empty()`.
pub fn render_mr_list_empty(frame: &mut Frame, area: Rect, colors: &crate::tui::theme::ThemeColors) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Length(8),
            Constraint::Percentage(45),
        ])
        .split(area);

    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("🔀  ", Style::default()),
            Span::styled("No merge requests yet", colors.accent().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   MRs appear here when you sync with a forge (GitLab / Forgejo)",
                colors.normal()),
        ]),
        Line::from(vec![
            Span::styled("   or create them via ", colors.normal()),
            Span::styled("prog mr create", colors.warning().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let paragraph = Paragraph::new(content)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(colors.border()),
        );

    frame.render_widget(paragraph, chunks[1]);
}
