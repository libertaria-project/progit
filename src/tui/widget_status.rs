//! Widget Status - System Status Bar
//!
//! Bottom bar showing git repo info, system metrics, and search/command input.

use crate::tui::app::{App, InputMode};
use crate::tui::theme::ThemeColors;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the bottom system status bar
pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = app.theme.colors();

    // Split into left (git/path info) and right (clock/search)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])
        .split(area);

    // LEFT: Git info + repo path
    let left_content = if let Some(ref repo) = app.repo_info {
        // Branch status style
        let branch_style = if repo.modified > 0 || repo.untracked > 0 {
            colors.warning()
        } else {
            colors.success()
        };

        let mut info = vec![
            Span::styled(" ", colors.accent()),
            Span::raw(" "),
            Span::styled("⎇ ", colors.dim()),
            Span::styled(&repo.branch, branch_style.add_modifier(Modifier::BOLD)),
        ];

        // Ahead/behind indicators
        if repo.ahead > 0 {
            info.push(Span::raw(" "));
            info.push(Span::styled(format!("↑{}", repo.ahead), colors.success()));
        }
        if repo.behind > 0 {
            info.push(Span::raw(" "));
            info.push(Span::styled(format!("↓{}", repo.behind), colors.warning()));
        }
        if repo.modified > 0 {
            info.push(Span::raw(" "));
            info.push(Span::styled(format!("●{}", repo.modified), colors.warning()));
        }

        // Repo path (shortened)
        info.push(Span::raw(" │ "));
        let short_path = if repo.path.len() > 40 {
            format!("…{}", &repo.path[repo.path.len()-37..])
        } else {
            repo.path.clone()
        };
        info.push(Span::styled(short_path, colors.dim()));

        Line::from(info)
    } else {
        Line::from(vec![
            Span::styled(" ", colors.dim()),
            Span::styled(" No git repository", colors.dim()),
        ])
    };

    let left = Paragraph::new(left_content).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(colors.border()),
    );

    frame.render_widget(left, chunks[0]);

    // RIGHT: Search/Command input OR clock + help hint
    let right_content = match app.input_mode {
        InputMode::Search => {
            Line::from(vec![
                Span::styled("/ ", colors.accent()),
                Span::raw(&app.search_query),
                Span::styled("█", colors.accent()),
            ])
        }
        InputMode::Command => {
            Line::from(vec![
                Span::styled(": ", colors.accent()),
                Span::raw(&app.command_input),
                Span::styled("█", colors.accent()),
            ])
        }
        _ => {
            // Show temporary status OR clock
            if let Some(msg) = app.get_status() {
                Line::from(Span::styled(format!(" {}", msg), colors.warning()))
            } else {
                let now = chrono::Local::now();
                Line::from(vec![
                    Span::styled(now.format("%H:%M").to_string(), colors.accent()),
                    Span::raw(" │ "),
                    Span::styled("? help", colors.dim()),
                ])
            }
        }
    };

    let right = Paragraph::new(right_content)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(colors.border()),
        )
        .alignment(ratatui::layout::Alignment::Right);

    frame.render_widget(right, chunks[1]);
}
