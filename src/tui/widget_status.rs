//! Widget Status - Status bar and sprint info
//!
//! Bottom bar with keybindings, search, and sprint timer.

use crate::tui::app::{App, InputMode};
use crate::tui::input::help_text;
use crate::tui::theme::ThemeColors;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the status bar
pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = app.theme.colors();

    // Split into left (status/search) and right (stats)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    // Left: Status message or help text or search input
    let left_content = match app.input_mode {
        InputMode::Search => {
            Line::from(vec![
                Span::styled(" / ", colors.accent()),
                Span::raw(&app.search_query),
                Span::styled("█", colors.accent()), // Cursor
            ])
        }
        InputMode::Command => {
             Line::from(vec![
                Span::styled(" : ", colors.accent()),
                Span::raw(&app.command_input),
                Span::styled("█", colors.accent()), // Cursor
            ])
        }
        _ => {
            // Check for temporary status message (auto-expires after 3s)
            if let Some(msg) = app.get_status() {
                Line::from(Span::styled(format!(" {} ", msg), colors.warning()))
            } else {
                // Default: Show useful project stats instead of just help
                let total = app.issues.len();
                let done = app.issues.iter().filter(|i| i.status == crate::issue::Status::Done).count();
                let in_progress = app.issues.iter().filter(|i| i.status == crate::issue::Status::InProgress).count();
                let blockers = app.blocker_count();
                
                let mut info = vec![
                    Span::styled(" 📊 ", colors.accent()),
                    Span::raw(format!("{}/{} done", done, total)),
                ];
                
                if in_progress > 0 {
                    info.push(Span::raw(" │ "));
                    info.push(Span::styled(format!("{} active", in_progress), colors.success()));
                }
                
                if blockers > 0 {
                    info.push(Span::raw(" │ "));
                    info.push(Span::styled(format!("🔥 {} blocked", blockers), colors.error()));
                }
                
                info.push(Span::raw(" │ "));
                info.push(Span::styled(help_text(app), colors.dim()));
                
                Line::from(info)
            }
        }
    };

    let left = Paragraph::new(left_content).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(colors.border()),
    );

    frame.render_widget(left, chunks[0]);

    // Right: Stats (velocity, blockers, sprint)
    let velocity = app.velocity();
    let blockers = app.blocker_count();

    let mut stats = vec![
        Span::styled(
            format!("⚡{} pts", velocity),
            colors.success().add_modifier(Modifier::BOLD),
        ),
    ];

    if blockers > 0 {
        stats.push(Span::raw(" │ "));
        stats.push(Span::styled(
            format!("🔥{}", blockers),
            colors.error(),
        ));
    }

    if let Some(sprint) = app.current_sprint {
        stats.push(Span::raw(" │ "));
        stats.push(Span::styled(
            format!("Sprint {}", sprint),
            colors.accent(),
        ));
    }

    // Add date/time
    let now = chrono::Local::now();
    stats.push(Span::raw(" │ "));
    stats.push(Span::styled(
        now.format("%H:%M").to_string(),
        colors.dim(),
    ));

    // Add sync indicator if we have remote links
    if app.sync_status.is_some() {
        stats.push(Span::raw(" │ "));
        stats.push(Span::styled(
            "🔄 Syncing",
            colors.warning(),
        ));
    }

    stats.push(Span::raw(" "));

    let right = Paragraph::new(Line::from(stats))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(colors.border()),
        )
        .alignment(ratatui::layout::Alignment::Right);

    frame.render_widget(right, chunks[1]);
}
