// SPDX-License-Identifier: EUPL-1.2
// Copyright (c) 2025 Markus Maiwald

//! Authentication notice modal.

use crate::tui::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Render a dismissible authentication notice above the active view.
pub fn render(frame: &mut Frame, app: &App) {
    let Some(notice) = app.auth_notice.as_ref() else {
        return;
    };

    let colors = app.theme.colors();
    let area = centered_rect(frame.area(), 70, 30);
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Authentication Required ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(Span::styled(
            "Remote sync needs a personal access token.",
            colors.header().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(notice.message.clone(), colors.normal())),
        Line::from(""),
        Line::from(vec![
            Span::styled("Esc", colors.accent().add_modifier(Modifier::BOLD)),
            Span::styled(" closes this notice. ", colors.dim()),
            Span::styled("Ctrl-C", colors.accent().add_modifier(Modifier::BOLD)),
            Span::styled(" exits ProGit.", colors.dim()),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Left);
    frame.render_widget(paragraph, inner);
}

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
