//! Command output modal.

use crate::tui::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    let Some(output) = app.command_output.as_ref() else {
        return;
    };

    let colors = app.theme.colors();
    let area = centered_rect(frame.area(), 82, 72);
    frame.render_widget(Clear, area);

    let border_style = if output.success {
        colors.success()
    } else {
        colors.error()
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(area);

    let title = output.title.as_deref().unwrap_or(" Command Output ");
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Command: ", colors.dim()),
            Span::styled(output.command.clone(), colors.normal()),
        ]),
        Line::from(vec![
            Span::styled("Status:  ", colors.dim()),
            Span::styled(
                output.status.clone(),
                if output.success {
                    colors.success()
                } else {
                    colors.error()
                },
            ),
        ]),
    ])
    .block(
        Block::default()
            .title(title)
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .border_type(BorderType::Rounded)
            .border_style(border_style),
    );
    frame.render_widget(header, chunks[0]);

    let lines = output_lines(output);
    let body = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((app.command_output_scroll as u16, 0))
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .border_style(border_style),
        );
    frame.render_widget(body, chunks[1]);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            " Enter/Esc/q ",
            colors.accent().add_modifier(Modifier::BOLD),
        ),
        Span::styled("close", colors.dim()),
        Span::styled("  j/k ", colors.accent().add_modifier(Modifier::BOLD)),
        Span::styled("scroll", colors.dim()),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
            .border_type(BorderType::Rounded)
            .border_style(border_style),
    );
    frame.render_widget(footer, chunks[2]);
}

fn output_lines(output: &crate::tui::app::CommandOutput) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    if output.stdout.trim().is_empty() && output.stderr.trim().is_empty() {
        lines.push(Line::from(Span::raw("(no output)")));
        return lines;
    }

    if !output.stdout.is_empty() {
        for line in output.stdout.lines() {
            lines.push(Line::from(Span::raw(line.to_string())));
        }
    }

    if !output.stderr.is_empty() {
        if !lines.is_empty() {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::raw("stderr:")));
        for line in output.stderr.lines() {
            lines.push(Line::from(Span::raw(line.to_string())));
        }
    }

    lines
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
