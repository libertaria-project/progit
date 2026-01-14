// SPDX-License-Identifier: EUPL-1.2
// Copyright (c) 2025 Markus Maiwald

//! Fuzzy Command Palette Widget
//!
//! The "Sublime Text Moment" - Ctrl+P to fuzzy search everything

// FuzzyMatch/FuzzySearcher unused here
use crate::tui::app::App;
use crate::tui::theme::ThemeColors;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

/// Render the fuzzy command palette overlay
pub fn render(frame: &mut Frame, app: &App, colors: &ThemeColors) -> Rect {
    let size = frame.area();

    // Center the palette (60% width, 50% height)
    let area = centered_rect(size, 60, 50);

    // Clear background
    frame.render_widget(ratatui::widgets::Clear, area);

    // Split into input and results
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input box
            Constraint::Min(1),    // Results list
        ])
        .split(area);

    // Render input box
    let input_text = if app.fuzzy_query.is_empty() {
        "Type to search issues, commands, files, commits..."
    } else {
        &app.fuzzy_query
    };

    let input_style = if app.fuzzy_query.is_empty() {
        colors.dim()
    } else {
        colors.normal()
    };

    let input = Paragraph::new(input_text).style(input_style).block(
        Block::default()
            .title(" 🔍 Fuzzy Search (Ctrl+P) ")
            .title_style(colors.accent().add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(colors.accent()),
    );

    frame.render_widget(input, chunks[0]);

    // Get search results
    let results = app.fuzzy_searcher.search(&app.fuzzy_query);

    // Render results list
    let items: Vec<ListItem> = results
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let is_selected = i == app.fuzzy_selected;
            let style = if is_selected {
                colors.selected()
            } else {
                colors.normal()
            };

            // Build display line with icon, text, and secondary info
            let mut spans = vec![
                Span::styled(format!("{} ", m.item.icon()), colors.accent()),
                Span::styled(m.item.display_text(), style),
            ];

            if let Some(secondary) = m.item.secondary_text() {
                spans.push(Span::styled(format!(" [{}]", secondary), colors.dim()));
            }

            // Add score for debugging (can remove later)
            spans.push(Span::styled(format!(" ({})", m.score), colors.dim()));

            ListItem::new(Line::from(spans)).style(style)
        })
        .collect();

    let results_count = results.len();
    let title = if results_count > 0 {
        format!(" {} Results ", results_count)
    } else if app.fuzzy_query.is_empty() {
        " Start typing to search... ".to_string()
    } else {
        " No matches found ".to_string()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .title_style(colors.header())
                .borders(Borders::ALL)
                .border_style(colors.border()),
        )
        .highlight_style(colors.selected().add_modifier(Modifier::BOLD));

    frame.render_widget(list, chunks[1]);

    // Show cursor in input box
    let cursor_x = chunks[0].x + app.fuzzy_query.len() as u16 + 1;
    let cursor_y = chunks[0].y + 1;
    frame.set_cursor_position((cursor_x, cursor_y));

    area
}

/// Helper to create centered rect
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
