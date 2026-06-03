// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2025 Markus Maiwald

//! Citadel Log Viewer Widget
//!
//! Modal overlay for displaying citadel command output (plan, validate, apply).

use crate::tui::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

/// Render the citadel log viewer as a centered modal overlay
pub fn render(frame: &mut Frame, app: &App) {
    let colors = app.theme.colors();
    let engine = &app.theme_engine;

    // Create centered modal area (80% width, 80% height)
    let area = centered_rect(80, 80, frame.area());

    // Clear background
    frame.render_widget(Clear, area);

    // Determine border color based on status
    let border_style = match &app.citadel_status {
        crate::citadel::CitadelStatus::Running(_) => {
            engine.get("pano.border.running", colors.accent())
        }
        crate::citadel::CitadelStatus::Success(_) => {
            engine.get("pano.border.success", colors.success())
        }
        crate::citadel::CitadelStatus::Error(_) => engine.get("pano.border.error", colors.error()),
        _ => engine.get("pano.border", colors.dim()),
    };

    // Build title based on status
    let title = match &app.citadel_status {
        crate::citadel::CitadelStatus::Running(msg) => format!(" 🔱 {} ", msg),
        crate::citadel::CitadelStatus::Success(_) => " ✓ Citadel Output ".to_string(),
        crate::citadel::CitadelStatus::Error(_) => " ✗ Citadel Output ".to_string(),
        _ => " 🔱 Citadel Output ".to_string(),
    };

    // Render block with scrollable content
    let output_lines: Vec<Line> = app
        .citadel_output
        .iter()
        .map(|line| {
            // Color-code based on content
            if line.contains("[ERR]") || line.contains("Error") {
                Line::from(Span::styled(line.as_str(), colors.error()))
            } else if line.contains("Plan:") || line.contains("+") {
                Line::from(Span::styled(line.as_str(), colors.success()))
            } else if line.contains("-") && !line.starts_with("---") {
                Line::from(Span::styled(line.as_str(), colors.warning()))
            } else if line.contains("~") {
                Line::from(Span::styled(line.as_str(), colors.warning()))
            } else {
                Line::from(line.as_str())
            }
        })
        .collect();

    let footer = Line::from(vec![
        Span::styled(" [Esc]", colors.accent().add_modifier(Modifier::BOLD)),
        Span::styled(" Close", colors.dim()),
    ]);

    let paragraph = Paragraph::new(output_lines)
        .block(
            Block::default()
                .title(Span::styled(
                    title,
                    engine.get("pano.title", colors.header().add_modifier(Modifier::BOLD)),
                ))
                .title_bottom(footer)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style),
        )
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);

    // TODO: Add scrolling support if output > area height
    // For MVP, showing last N lines is acceptable
}

/// Helper function to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
