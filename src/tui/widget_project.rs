// SPDX-License-Identifier: EUPL-1.2
// Copyright (c) 2026 Markus Maiwald

//! Read-only project wiki and issue overlays.

use crate::tui::app::App;
use crate::tui::markdown::render_markdown;
use crate::tui::theme::ThemeColors;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// Render repository-owned wiki pages from `.project/wiki/manifest.kdl`.
pub fn render_wiki(frame: &mut Frame, app: &App, colors: &ThemeColors) {
    let area = centered_rect(frame.area(), 82, 76);
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Project Wiki  h/l page  j/k scroll  q close ")
        .title_style(colors.accent().add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(colors.accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(view) = &app.project_wiki_view else {
        render_empty(frame, inner, "No project wiki loaded", colors);
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(20)])
        .split(inner);

    let page_items: Vec<ListItem> = view
        .pages
        .iter()
        .enumerate()
        .map(|(idx, page)| {
            let selected = idx == app.project_wiki_page;
            let prefix = if selected { "> " } else { "  " };
            let style = if selected {
                colors.selected().add_modifier(Modifier::BOLD)
            } else {
                colors.normal()
            };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, colors.accent()),
                Span::styled(page.name.clone(), style),
            ]))
        })
        .collect();

    let pages = List::new(page_items).block(
        Block::default()
            .title(format!(" Pages ({}) ", view.pages.len()))
            .borders(Borders::ALL)
            .border_style(colors.border()),
    );
    frame.render_widget(pages, chunks[0]);

    if let Some(page) = view.pages.get(app.project_wiki_page) {
        let body = render_markdown(&page.content, colors.normal());
        let content = Paragraph::new(body)
            .block(
                Block::default()
                    .title(format!(" {} - {} ", page.name, page.title))
                    .borders(Borders::ALL)
                    .border_style(colors.border()),
            )
            .scroll((app.project_wiki_scroll, 0))
            .wrap(Wrap { trim: false });
        frame.render_widget(content, chunks[1]);
    }
}

/// Render repository-owned issue files from `.project/issues/*.json`.
pub fn render_issues(frame: &mut Frame, app: &App, colors: &ThemeColors) {
    let area = centered_rect(frame.area(), 76, 64);
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Project Issues  j/k navigate  Enter open loaded issue  q close ")
        .title_style(colors.accent().add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(colors.accent());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(view) = &app.project_issues_view else {
        render_empty(frame, inner, "No project issues loaded", colors);
        return;
    };

    if view.issues.is_empty() {
        render_empty(
            frame,
            inner,
            "No .project/issues/*.json files found",
            colors,
        );
        return;
    }

    let items: Vec<ListItem> = view
        .issues
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let issue = &entry.issue;
            let selected = idx == app.project_issue_selected;
            let style = if selected {
                colors.selected().add_modifier(Modifier::BOLD)
            } else if issue.is_blocker() {
                colors.error().add_modifier(Modifier::BOLD)
            } else {
                colors.normal()
            };
            let tags = if issue.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", issue.tags.join(","))
            };
            let prefix = if selected { "> " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, colors.accent()),
                Span::styled(issue.short_id().to_string(), colors.dim()),
                Span::raw("  "),
                Span::styled(issue.title.clone(), style),
                Span::styled(format!("  {}", issue.status.as_str()), colors.dim()),
                Span::styled(tags, colors.accent()),
                Span::styled(format!("  {}", entry.path.display()), colors.dim()),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(format!(" Issue Files ({}) ", view.issues.len()))
            .borders(Borders::ALL)
            .border_style(colors.border()),
    );
    frame.render_widget(list, inner);
}

fn render_empty(frame: &mut Frame, area: Rect, message: &str, colors: &ThemeColors) {
    let text = Paragraph::new(message).style(colors.dim()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(colors.border()),
    );
    frame.render_widget(text, area);
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
