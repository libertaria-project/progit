// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2025 Markus Maiwald

//! Widget Plugins — plugin manager modal overlay.
//!
//! Shows installed plugins with metadata (name, version, hooks) and
//! their quarantine state. Quarantined plugins are flagged in red with
//! the failure reason; pressing `u` while one is highlighted clears
//! its quarantine.
//!
//! Opened via `P` (or `Q` as alias), navigated with j/k, closed with Esc.

use crate::tui::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use std::collections::HashMap;

/// Render the plugin manager modal (centered overlay)
pub fn render(frame: &mut Frame, app: &App) {
    let colors = app.theme.colors();
    let size = frame.area();

    // Center the modal (65% width, 60% height)
    let area = centered_rect(size, 65, 60);

    // Clear background for overlay
    frame.render_widget(Clear, area);

    let mut lines: Vec<Line> = Vec::new();

    match app.plugin_manager.as_ref() {
        Some(pm) if pm.count() > 0 => {
            let infos = pm.plugin_info();
            let count = infos.len();
            let selected = app.plugin_selected.min(count.saturating_sub(1));

            // Snapshot quarantine state once per render.
            let quarantined: HashMap<&str, &str> = pm.quarantined_plugins().collect();

            let header_text = if quarantined.is_empty() {
                format!("{} plugin{} loaded", count, if count == 1 { "" } else { "s" })
            } else {
                format!(
                    "{} plugin{} loaded · {} quarantined",
                    count,
                    if count == 1 { "" } else { "s" },
                    quarantined.len()
                )
            };
            lines.push(Line::from(vec![Span::styled(
                header_text,
                colors.header().add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(""));

            for (idx, meta) in infos.iter().enumerate() {
                let is_selected = idx == selected;
                let cursor = if is_selected { "▶ " } else { "  " };
                let is_quarantined = quarantined.contains_key(meta.name.as_str());

                let name_style = if is_quarantined {
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                } else if is_selected {
                    colors.selected().add_modifier(Modifier::BOLD)
                } else {
                    colors.accent().add_modifier(Modifier::BOLD)
                };
                let row_style = if is_selected { colors.selected() } else { colors.normal() };

                let mut row = vec![Span::styled(cursor, colors.accent())];
                if is_quarantined {
                    row.push(Span::styled(
                        "[QUARANTINED] ",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ));
                }
                row.push(Span::styled(meta.name.clone(), name_style));
                row.push(Span::styled("  v", colors.dim()));
                row.push(Span::styled(meta.version.clone(), colors.dim()));
                row.push(Span::styled("  by ", colors.dim()));
                row.push(Span::styled(meta.author.clone(), row_style));
                lines.push(Line::from(row));

                if let Some(reason) = quarantined.get(meta.name.as_str()) {
                    lines.push(Line::from(vec![
                        Span::styled("    ", colors.dim()),
                        Span::styled("reason: ", Style::default().fg(Color::Red)),
                        Span::styled((*reason).to_string(), Style::default().fg(Color::Red)),
                    ]));
                }

                if !meta.description.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("    ", colors.dim()),
                        Span::styled(meta.description.clone(), colors.dim()),
                    ]));
                }

                let hooks_label = if meta.hooks.is_empty() {
                    "(no hooks)".to_string()
                } else {
                    meta.hooks
                        .iter()
                        .map(|h| format!("{:?}", h))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                lines.push(Line::from(vec![
                    Span::styled("    hooks: ", colors.dim()),
                    Span::styled(hooks_label, colors.normal()),
                ]));
                lines.push(Line::from(""));
            }

            // Help line. Mention `u` only when there's something to clear.
            let mut help: Vec<Span> = vec![
                Span::styled("j/k", colors.accent()),
                Span::styled(" navigate  ", colors.dim()),
            ];
            if !quarantined.is_empty() {
                help.push(Span::styled("u", colors.accent()));
                help.push(Span::styled(" unquarantine  ", colors.dim()));
            }
            help.push(Span::styled("Esc/P/Q/q", colors.accent()));
            help.push(Span::styled(" close", colors.dim()));
            lines.push(Line::from(help));
        }
        _ => {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "No plugins installed",
                colors.header().add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Install plugins from the marketplace:",
                colors.normal(),
            )]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  prog plugin install ", colors.dim()),
                Span::styled("<name>", colors.accent()),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  prog plugin list", colors.dim()),
                Span::styled("       ", colors.dim()),
                Span::styled("# browse marketplace", colors.dim()),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Plugins live in .project/plugins/ as .lua files or directories.",
                colors.dim(),
            )]));
        }
    }

    let content = Paragraph::new(lines).block(
        Block::default()
            .title(Span::styled(
                " Plugins ",
                colors.accent().add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(colors.accent()),
    );

    frame.render_widget(content, area);
}

/// Create a centered rect
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
