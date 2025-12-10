//! Widget Git Bar - Top bar showing repository info
//!
//! Displays branch, remote, and sync status.

use super::repository::{format_remote_url, RepoInfo};
use crate::tui::theme::ThemeColors;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the git bar at the top
/// Render the git bar, return (branch_area, remote_area)
pub fn render(frame: &mut Frame, area: Rect, repo: Option<&RepoInfo>, colors: &ThemeColors, dropdown_open: bool) -> (Rect, Rect) {
    if let Some(repo) = repo {
        render_repo_bar(frame, area, repo, colors, dropdown_open)
    } else {
        render_no_repo(frame, area, colors);
        (Rect::default(), Rect::default())
    }
}

/// Render bar when repository is detected
fn render_repo_bar(frame: &mut Frame, area: Rect, repo: &RepoInfo, colors: &ThemeColors, dropdown_open: bool) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(3),      // Git icon
            Constraint::Percentage(25), // Branch
            Constraint::Percentage(45), // Remote (clickable)
            Constraint::Percentage(30), // Status
        ])
        .split(area);
        




    // Git icon
    let icon = Paragraph::new(" ")
        .style(colors.accent())
        .block(Block::default().borders(Borders::BOTTOM).border_style(colors.border()));
    frame.render_widget(icon, chunks[0]);

    // Branch name
    let branch_style = if repo.modified > 0 || repo.untracked > 0 {
        colors.warning()
    } else {
        colors.success()
    };
    let branch = Paragraph::new(Line::from(vec![
        Span::styled("⎇ ", colors.dim()),
        Span::styled(&repo.branch, branch_style.add_modifier(Modifier::BOLD)),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).border_style(colors.border()));
    frame.render_widget(branch, chunks[1]);

    // Remote (with dropdown indicator - clickable)
    let remote_display = if let Some(ref url) = repo.remote_url {
        format_remote_url(url)
    } else {
        "No remote".to_string()
    };

    // Shorten local path for display
    let local_display = {
        let path = &repo.path;
        if path.len() > 30 {
            format!("...{}", &path[path.len()-27..])
        } else {
            path.clone()
        }
    };

    let dropdown_indicator = if dropdown_open { " ▲" } else { " ▼" };
    let remote = Paragraph::new(Line::from(vec![
        Span::styled(&local_display, colors.dim()),
        Span::raw(" → "),
        Span::styled(&remote_display, colors.accent().add_modifier(Modifier::UNDERLINED)),
        Span::styled(dropdown_indicator, colors.dim()),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).border_style(colors.border()));
    frame.render_widget(remote, chunks[2]);

    // Status (ahead/behind, modified)
    let mut status_spans = Vec::new();

    if repo.ahead > 0 {
        status_spans.push(Span::styled(format!("↑{}", repo.ahead), colors.success()));
        status_spans.push(Span::raw(" "));
    }
    if repo.behind > 0 {
        status_spans.push(Span::styled(format!("↓{}", repo.behind), colors.warning()));
        status_spans.push(Span::raw(" "));
    }
    if repo.modified > 0 {
        status_spans.push(Span::styled(format!("●{}", repo.modified), colors.warning()));
        status_spans.push(Span::raw(" "));
    }
    if repo.untracked > 0 {
        status_spans.push(Span::styled(format!("+{}", repo.untracked), colors.dim()));
    }

    if status_spans.is_empty() {
        status_spans.push(Span::styled("✓ synced", colors.success()));
    }

    let status = Paragraph::new(Line::from(status_spans))
        .alignment(Alignment::Right)
        .block(Block::default().borders(Borders::BOTTOM).border_style(colors.border()));
    frame.render_widget(status, chunks[3]);
    
    (chunks[1], chunks[2])
}

/// Render bar when no repository detected
fn render_no_repo(frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let content = Paragraph::new(Line::from(vec![
        Span::styled(" ", colors.dim()),
        Span::styled(" No git repository ", colors.dim()),
        Span::styled("(click to connect)", colors.accent()),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).border_style(colors.border()));
    frame.render_widget(content, area);
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
