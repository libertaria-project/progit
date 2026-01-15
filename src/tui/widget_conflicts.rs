// SPDX-License-Identifier: EUPL-1.2
// Copyright (c) 2025 Markus Maiwald

//! Conflict Resolution Widget
//! 
//! [ARCH] Visual representation of conflicting hunks between virtual branches.
//! Displays side-by-side comparison and provides resolution actions.

use crate::tui::app::App;
use crate::tui::theme::ThemeColors;
use crate::virtual_branch::HunkRef;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};

/// Render the conflict resolution modal
pub fn render(frame: &mut Frame, app: &App) {
    let colors = app.theme.colors();
    
    // Get current branch and its conflicts
    let (current_branch, conflicts) = match &app.vbranch_manager {
        Some(manager) => {
            let branches = manager.list();
            if let Some(branch) = branches.get(app.vbranch_selected) {
                let all_conflicts = manager.detect_conflicts();
                let branch_conflicts = all_conflicts.get(&branch.id).cloned().unwrap_or_default();
                (Some((*branch).clone()), branch_conflicts)
            } else {
                (None, Vec::new())
            }
        }
        None => (None, Vec::new()),
    };
    
    if conflicts.is_empty() || current_branch.is_none() {
        render_no_conflicts(frame, &colors);
        return;
    }
    
    let current = current_branch.unwrap();
    
    // Create centered modal (80% width, 80% height)
    let area = centered_rect(85, 85, frame.area());
    
    // Clear background
    let block = Block::default()
        .title(format!(" 🔀 Conflicts: {} ", current.name))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(Color::Black));
    
    let inner = block.inner(area);
    frame.render_widget(block, area);
    
    // Split: conflict list (30%) | details (70%)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(inner);
    
    // Render conflict list
    render_conflict_list(frame, chunks[0], &conflicts, &colors, app);
    
    // Render conflict details
    render_conflict_details(frame, chunks[1], &current, &conflicts, &colors, app);
}

/// Render the list of conflicting branches
fn render_conflict_list(
    frame: &mut Frame,
    area: Rect,
    conflict_branch_ids: &[String],
    colors: &ThemeColors,
    app: &App,
) {
    let items: Vec<ListItem> = conflict_branch_ids
        .iter()
        .enumerate()
        .filter_map(|(idx, branch_id)| {
            let manager = app.vbranch_manager.as_ref()?;
            let branch = manager.get(branch_id)?;
            
            let prefix = if idx == 0 { "▶ " } else { "  " };
            let style = if idx == 0 {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            
            Some(ListItem::new(format!("{}{}", prefix, branch.name)).style(style))
        })
        .collect();
    
    let block = Block::default()
        .title(" Conflicting Branches ")
        .borders(Borders::ALL)
        .border_style(colors.border());
    
    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

/// Render detailed conflict information
fn render_conflict_details(
    frame: &mut Frame,
    area: Rect,
    current_branch: &crate::virtual_branch::VirtualBranch,
    conflict_branch_ids: &[String],
    colors: &ThemeColors,
    app: &App,
) {
    // Find overlapping hunks
    let mut overlapping_hunks: Vec<(HunkRef, Vec<String>)> = Vec::new();
    
    if let Some(manager) = &app.vbranch_manager {
        for hunk in &current_branch.owned_hunks {
            let mut conflicting_branches = Vec::new();
            
            for conflict_id in conflict_branch_ids {
                if let Some(other_branch) = manager.get(conflict_id) {
                    for other_hunk in &other_branch.owned_hunks {
                        if hunk.overlaps_with(other_hunk) {
                            conflicting_branches.push(other_branch.name.clone());
                            break;
                        }
                    }
                }
            }
            
            if !conflicting_branches.is_empty() {
                overlapping_hunks.push((hunk.clone(), conflicting_branches));
            }
        }
    }
    
    if overlapping_hunks.is_empty() {
        let msg = Paragraph::new("No overlapping hunks found")
            .style(colors.dim())
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(" Details ")
                    .borders(Borders::ALL)
                    .border_style(colors.border()),
            );
        frame.render_widget(msg, area);
        return;
    }
    
    // Display overlapping hunks
    let mut lines = vec![
        Line::from(Span::styled(
            format!("Found {} conflicting hunk(s):", overlapping_hunks.len()),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    
    for (hunk, branches) in overlapping_hunks.iter().take(10) {
        lines.push(Line::from(vec![
            Span::styled("📄 ", colors.accent()),
            Span::styled(&hunk.file_path, Style::default().fg(Color::Cyan)),
            Span::raw(format!(" (lines {}-{})", hunk.new_start, hunk.new_start + hunk.new_count)),
        ]));
        
        lines.push(Line::from(vec![
            Span::raw("   Conflicts with: "),
            Span::styled(branches.join(", "), Style::default().fg(Color::Red)),
        ]));
        
        lines.push(Line::from(""));
    }
    
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("💡 ", colors.accent()),
        Span::styled("Use ", colors.dim()),
        Span::styled("'m'", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::styled(" to move hunks between branches", colors.dim()),
    ]));
    
    let block = Block::default()
        .title(" Conflict Details ")
        .borders(Borders::ALL)
        .border_style(colors.border());
    
    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

/// Render "no conflicts" message
fn render_no_conflicts(frame: &mut Frame, colors: &ThemeColors) {
    let area = centered_rect(50, 30, frame.area());
    
    let block = Block::default()
        .title(" No Conflicts ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(colors.success())
        .style(Style::default().bg(Color::Black));
    
    let inner = block.inner(area);
    frame.render_widget(block, area);
    
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "✓ No conflicts detected",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("This branch has no overlapping hunks", colors.dim())),
        Line::from(Span::styled("with other branches.", colors.dim())),
    ];
    
    let para = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(para, inner);
}

/// Helper to create a centered rect
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
