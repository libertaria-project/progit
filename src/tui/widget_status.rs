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
pub fn render(frame: &mut Frame, area: Rect, app: &mut App) -> Option<Rect> {
    // Get status message first (requires mutable borrow) to avoid conflict later
    let status_msg = app.get_status();
    
    let colors = app.theme.colors();
    let engine = &app.theme_engine;

    // Split into left (git/path info) and right (clock/search)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])
        .split(area);

    let border_style = engine.get("status.border", colors.border());

    // LEFT: Git info + repo path
    let left_content = if let Some(ref repo) = app.repo_info {
        // Branch status style
        let is_dirty = repo.modified > 0 || repo.untracked > 0;
        let branch_base = if is_dirty { colors.warning() } else { colors.success() };
        
        let branch_style = if is_dirty {
            engine.get("status.branch.dirty", branch_base)
        } else {
            engine.get("status.branch.clean", branch_base)
        };

        let mut info = vec![
            Span::styled(" ", colors.accent()), // Spacer
            Span::raw(" "),
            Span::styled("⎇ ", engine.get("status.branch.icon", colors.dim())),
            Span::styled(&repo.branch, branch_style.add_modifier(Modifier::BOLD)),
        ];

        // Ahead/behind indicators
        if repo.ahead > 0 {
            info.push(Span::raw(" "));
            info.push(Span::styled(
                format!("↑{}", repo.ahead), 
                engine.get("status.remote.ahead", colors.success())
            ));
        }
        if repo.behind > 0 {
            info.push(Span::raw(" "));
            info.push(Span::styled(
                format!("↓{}", repo.behind), 
                engine.get("status.remote.behind", colors.warning())
            ));
        }
        if repo.modified > 0 {
            info.push(Span::raw(" "));
            info.push(Span::styled(
                format!("●{}", repo.modified), 
                engine.get("status.file.modified", colors.warning())
            ));
        }

        // Repo path (shortened)
        info.push(Span::styled(" │ ", engine.get("status.separator", colors.dim())));
        let short_path = if repo.path.len() > 40 {
            format!("…{}", &repo.path[repo.path.len()-37..])
        } else {
            repo.path.clone()
        };
        info.push(Span::styled(short_path, engine.get("status.path", colors.dim())));

        Line::from(info)
    } else {
        Line::from(vec![
            Span::styled(" ", colors.dim()),
            Span::styled(" No git repository", engine.get("status.norepo", colors.dim())),
        ])
    };

    let left = Paragraph::new(left_content).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(border_style),
    );

    frame.render_widget(left, chunks[0]);

    // RIGHT: Search/Command input OR clock + help hint
    let right_content = match app.input_mode {
        InputMode::Search => {
            Line::from(vec![
                Span::styled("/ ", engine.get("status.mode.search", colors.accent())),
                Span::raw(&app.search_query),
                Span::styled("█", engine.get("status.cursor", colors.accent())),
            ])
        }
        InputMode::Command => {
            Line::from(vec![
                Span::styled(": ", engine.get("status.mode.command", colors.accent())),
                Span::raw(&app.command_input),
                Span::styled("█", engine.get("status.cursor", colors.accent())),
            ])
        }
        _ => {
            // Show temporary status OR clock
            if let Some(ref msg) = status_msg {
                Line::from(Span::styled(format!(" {}", msg), engine.get("status.message", colors.warning())))
            } else {
                let now = chrono::Local::now();
                Line::from(vec![
                    Span::styled(now.format("%H:%M").to_string(), engine.get("status.clock", colors.accent())),
                    Span::styled(" │ ", engine.get("status.separator", colors.dim())),
                    Span::styled("? help", engine.get("status.help", colors.dim())),
                ])
            }
        }
    };

    let right = Paragraph::new(right_content)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(border_style),
        )
        .alignment(ratatui::layout::Alignment::Right);

    frame.render_widget(right, chunks[1]);
    
    // Calculate help icon area if visible
    // "HH:MM | ? help" is shown when not searching/commanding and no status msg
    if !matches!(app.input_mode, InputMode::Search | InputMode::Command) && status_msg.is_none() {
        // Approximate area: last 6 chars of the right chunk
        let help_width: u16 = 6;
        if chunks[1].width >= help_width + 2 {
            return Some(Rect {
                x: chunks[1].x + chunks[1].width - help_width - 1,
                y: chunks[1].y + 1, // +1 for border
                width: help_width,
                height: 1,
            });
        }
    }
    
    None
}

/// Get context-aware help text
pub fn help_text(app: &App) -> String {
    match app.view_mode {
        crate::tui::ViewMode::List => {
             "n:new  space:status  e:edit  /:search  s:sync  t:theme  ?:help".to_string()
        }
        crate::tui::ViewMode::Kanban => {
             "n:new  H/L:move  space:status  enter:details  s:sync  ?:help".to_string()
        }
    }
}
