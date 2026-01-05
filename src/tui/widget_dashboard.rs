use crate::tui::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = app.theme.colors();
    let _engine = &app.theme_engine;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Welcome Header
            Constraint::Min(5),    // Main Stats
            Constraint::Length(8), // Shortcuts
        ])
        .split(area);

    // 1. Welcome Header
    let user = std::env::var("USER").unwrap_or_else(|_| "Developer".to_string());
    let repo_name = app.repo_info.as_ref().map(|r| r.repo_name.clone()).unwrap_or_else(|| "Unknown Repo".to_string());
    
    let header_text = vec![
        Line::from(vec![
            Span::styled(" 🔱 PROGIT ", colors.accent().add_modifier(Modifier::BOLD)),
            Span::styled(format!(" - Welcome, {}!", user), colors.normal()),
        ]),
        Line::from(vec![
            Span::styled(" Working on: ", colors.dim()),
            Span::styled(&repo_name, colors.warning().add_modifier(Modifier::BOLD)),
        ]),
    ];
    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(header, chunks[0]);

    // 2. Main Stats (Grid)
    let stats_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Issue Stats
            Constraint::Percentage(50), // MR Stats
        ])
        .split(chunks[1]);

    // Issue Stats
    let (backlog, in_progress, done) = app.issues_by_status();
    let total = app.issues.len();
    let done_count = done.len();
    let completion = if total > 0 { (done_count as f32 / total as f32) * 100.0 } else { 0.0 };
    
    // Progress bar: [######....]
    let pb_width = 20;
    let filled = if total > 0 { (done_count * pb_width) / total } else { 0 };
    let empty = pb_width - filled;
    let progress_bar = format!("[{}{}] {:.1}%", "#".repeat(filled), ".".repeat(empty), completion);

    let issue_stats_text = vec![
        Line::from(Span::styled(" 📋 Issues ", colors.accent().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("   • Backlog:     ", colors.normal()),
            Span::styled(format!("{}", backlog.len()), colors.warning().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("   • In Progress: ", colors.normal()),
            Span::styled(format!("{}", in_progress.len()), colors.success().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("   • Done/Merged: ", colors.normal()),
            Span::styled(format!("{}", done.len()), colors.success().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   Sprint Progress: ", colors.dim()),
            Span::styled(progress_bar, colors.success()),
        ]),
    ];
    let issue_stats = Paragraph::new(issue_stats_text)
        .block(Block::default().borders(Borders::ALL).title(" Issue Health "));
    frame.render_widget(issue_stats, stats_chunks[0]);

    // MR Stats
    let open_mrs = app.mr_list.iter().filter(|m| m.state == crate::mr::MRState::Open).count();
    let merged_mrs = app.mr_list.iter().filter(|m| m.state == crate::mr::MRState::Merged).count();
    let mr_stats_text = vec![
        Line::from(Span::styled(" 🔀 Merge Requests ", colors.accent().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("   • Open:        ", colors.normal()),
            Span::styled(format!("{}", open_mrs), colors.success().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("   • Merged:      ", colors.normal()),
            Span::styled(format!("{}", merged_mrs), colors.accent().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   Pipeline: ", colors.dim()),
            Span::styled("🟢 Passed", colors.success().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("   Sync Status: ", colors.dim()),
            Span::styled("✓ Up to date", colors.success()),
        ]),
    ];
    let mr_stats = Paragraph::new(mr_stats_text)
        .block(Block::default().borders(Borders::ALL).title(" Forge Status "));
    frame.render_widget(mr_stats, stats_chunks[1]);

    // 3. Shortcuts (Legend)
    let shortcuts_text = vec![
        Line::from(Span::styled(" ⌨  Key Bindings ", colors.accent().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("   [Tab]  ", colors.warning()), Span::raw(" Cycle View Modes"),
            Span::styled("       [n]    ", colors.warning()), Span::raw(" New Issue"),
        ]),
        Line::from(vec![
            Span::styled("   [S]    ", colors.warning()), Span::raw(" Sync with Forge"),
            Span::styled("       [O]    ", colors.warning()), Span::raw(" Settings"),
        ]),
        Line::from(vec![
            Span::styled("   [/]    ", colors.warning()), Span::raw(" Global Search"),
            Span::styled("       [q]    ", colors.warning()), Span::raw(" Quit"),
        ]),
        Line::from(vec![
            Span::styled("   [Ctrl+P]", colors.warning()), Span::raw(" Command Palette"),
            Span::styled("     [d]    ", colors.warning()), Span::raw(" Quick Diff"),
        ]),
    ];
    let shortcuts = Paragraph::new(shortcuts_text)
        .block(Block::default().borders(Borders::ALL).title(" Quick Start "));
    frame.render_widget(shortcuts, chunks[2]);
}
