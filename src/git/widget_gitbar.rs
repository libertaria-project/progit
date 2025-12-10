//! Widget Git Bar - Project Management Metrics Bar
//!
//! Top bar displaying PO/PM metrics: velocity, progress, blockers, sprint.

use super::repository::{format_remote_url, RepoInfo};
use crate::tui::theme::ThemeColors;
use crate::issue::Status;
use crate::tui::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the PO metrics bar at the top
/// Returns empty rects for branch/remote areas (legacy compatibility)
pub fn render(frame: &mut Frame, area: Rect, repo: Option<&RepoInfo>, colors: &ThemeColors, _dropdown_open: bool) -> (Rect, Rect) {
    render_pm_metrics_bar(frame, area, colors);
    (Rect::default(), Rect::default())
}

/// Render the PO/PM metrics bar
fn render_pm_metrics_bar(frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20), // Velocity
            Constraint::Percentage(25), // Done/Total progress
            Constraint::Percentage(20), // Active items
            Constraint::Percentage(20), // Blockers
            Constraint::Percentage(15), // Sprint info
        ])
        .split(area);

    // We need access to App to get issue data
    // For now, render placeholder - we'll update the signature in tui.rs
    
    // Velocity section
    let velocity_widget = Paragraph::new("⚡ -- pts")
        .style(colors.success().add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::BOTTOM).border_style(colors.border()));
    frame.render_widget(velocity_widget, chunks[0]);

    // Progress section  
    let progress_widget = Paragraph::new("📊 --/-- done")
        .style(colors.accent())
        .block(Block::default().borders(Borders::BOTTOM).border_style(colors.border()));
    frame.render_widget(progress_widget, chunks[1]);

    // Active items
    let active_widget = Paragraph::new("🔄 -- active")
        .style(colors.normal())
        .block(Block::default().borders(Borders::BOTTOM).border_style(colors.border()));
    frame.render_widget(active_widget, chunks[2]);

    // Blockers
    let blocker_widget = Paragraph::new("🔥 -- blocked")
        .style(colors.error())
        .block(Block::default().borders(Borders::BOTTOM).border_style(colors.border()));
    frame.render_widget(blocker_widget, chunks[3]);

    // Sprint
    let sprint_widget = Paragraph::new("Sprint --")
        .style(colors.dim())
        .alignment(Alignment::Right)
        .block(Block::default().borders(Borders::BOTTOM).border_style(colors.border()));
    frame.render_widget(sprint_widget, chunks[4]);
}

/// Render PO metrics bar with actual data from App
pub fn render_with_app(frame: &mut Frame, area: Rect, app: &App, colors: &ThemeColors) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20), // Velocity
            Constraint::Percentage(25), // Done/Total progress
            Constraint::Percentage(20), // Active items
            Constraint::Percentage(20), // Blockers
            Constraint::Percentage(15), // Sprint info
        ])
        .split(area);

    // Velocity
    let velocity = app.velocity();
    let velocity_widget = Paragraph::new(Line::from(vec![
        Span::styled("⚡ ", colors.success()),
        Span::styled(format!("{} pts", velocity), colors.success().add_modifier(Modifier::BOLD)),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).border_style(colors.border()));
    frame.render_widget(velocity_widget, chunks[0]);

    // Progress (Done/Total)
    let total = app.issues.len();
    let done = app.issues.iter().filter(|i| i.status == Status::Done).count();
    let progress_pct = if total > 0 { (done * 100) / total } else { 0 };
    let progress_widget = Paragraph::new(Line::from(vec![
        Span::styled("📊 ", colors.accent()),
        Span::styled(format!("{}/{}", done, total), colors.accent().add_modifier(Modifier::BOLD)),
        Span::styled(format!(" ({}%)", progress_pct), colors.dim()),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).border_style(colors.border()));
    frame.render_widget(progress_widget, chunks[1]);

    // Active items
    let in_progress = app.issues.iter().filter(|i| i.status == Status::InProgress).count();
    let active_style = if in_progress > 0 { colors.success() } else { colors.dim() };
    let active_widget = Paragraph::new(Line::from(vec![
        Span::styled("🔄 ", colors.accent()),
        Span::styled(format!("{} active", in_progress), active_style),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).border_style(colors.border()));
    frame.render_widget(active_widget, chunks[2]);

    // Blockers
    let blockers = app.blocker_count();
    let blocker_style = if blockers > 0 { colors.error().add_modifier(Modifier::BOLD) } else { colors.dim() };
    let blocker_widget = Paragraph::new(Line::from(vec![
        Span::styled("🔥 ", colors.error()),
        Span::styled(format!("{} blocked", blockers), blocker_style),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).border_style(colors.border()));
    frame.render_widget(blocker_widget, chunks[3]);

    // Sprint info
    let sprint_text = if let Some(sprint) = app.current_sprint {
        format!("Sprint {}", sprint)
    } else {
        "No Sprint".to_string()
    };
    let sprint_widget = Paragraph::new(sprint_text)
        .style(colors.accent())
        .alignment(Alignment::Right)
        .block(Block::default().borders(Borders::BOTTOM).border_style(colors.border()));
    frame.render_widget(sprint_widget, chunks[4]);

    (Rect::default(), Rect::default())
}

/// Render the remote dropdown menu
pub fn render_dropdown(frame: &mut Frame, area: Rect, repo: &RepoInfo, selected: usize, colors: &ThemeColors) {
    use ratatui::widgets::Clear;
    frame.render_widget(Clear, area);
    
    let items: Vec<Line> = repo
        .remotes
        .iter()
        .enumerate()
        .map(|(i, remote)| {
            let prefix = if i == selected { "▶ " } else { "  " };
            let style = if i == selected {
                colors.selected()
            } else {
                colors.normal()
            };
            Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(&remote.name, style.add_modifier(Modifier::BOLD)),
                Span::styled(" → ", colors.dim()),
                Span::styled(format_remote_url(&remote.url), style),
            ])
        })
        .collect();

    // Add "Add new remote" option
    let mut all_items = items;
    all_items.push(Line::from(vec![
        Span::styled("  ", colors.normal()),
        Span::styled("+ Add new remote...", colors.accent()),
    ]));

    let dropdown = Paragraph::new(all_items)
        .block(
            Block::default()
                .title(" Remotes ")
                .borders(Borders::ALL)
                .border_style(colors.accent()),
        );

    frame.render_widget(dropdown, area);
}

/// Render the branch dropdown menu
pub fn render_branch_dropdown(frame: &mut Frame, area: Rect, repo: &RepoInfo, selected: usize, colors: &ThemeColors) {
    // Clear background first to prevent bleed-through
    use ratatui::widgets::Clear;
    frame.render_widget(Clear, area);
    
    let items: Vec<Line> = repo
        .branches
        .iter()
        .enumerate()
        .map(|(i, branch)| {
            let prefix = if i == selected { "▶ " } else { "  " };
            let style = if i == selected {
                colors.selected()
            } else {
                colors.normal()
            };
            // Highlight current branch
            let name_style = if branch == &repo.branch {
                style.add_modifier(Modifier::BOLD).add_modifier(Modifier::UNDERLINED)
            } else {
                style
            };
            Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(branch, name_style),
            ])
        })
        .collect();

    // Add "New branch" option
    let mut all_items = items;
    let new_idx = repo.branches.len();
    let prefix = if selected == new_idx { "▶ " } else { "  " };
    let style = if selected == new_idx { colors.selected() } else { colors.normal() };
    
    all_items.push(Line::from(vec![
        Span::styled(prefix, style),
        Span::styled("+ New Branch...", colors.accent()),
    ]));

    let dropdown = Paragraph::new(all_items)
        .block(
            Block::default()
                .title(" Branches ")
                .borders(Borders::ALL)
                .border_style(colors.accent()),
        );

    frame.render_widget(dropdown, area);
}

/// Render the branch name input field
pub fn render_branch_input(frame: &mut Frame, area: Rect, input: &str, colors: &ThemeColors) {
    let content = Line::from(vec![
        Span::styled("New branch: ", colors.dim()),
        Span::styled(input, colors.accent().add_modifier(Modifier::BOLD)),
        Span::styled("▌", colors.accent()), // Cursor
    ]);
    
    let input_box = Paragraph::new(content)
        .block(
            Block::default()
                .title(" Create Branch ")
                .borders(Borders::ALL)
                .border_style(colors.success()),
        );

    frame.render_widget(input_box, area);
}
