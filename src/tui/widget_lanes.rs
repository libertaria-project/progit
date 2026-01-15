// SPDX-License-Identifier: EUPL-1.2
// Copyright (c) 2025 Markus Maiwald

//! Widget Lanes - Virtual Branch Lanes View
//!
//! [ARCH] GitButler-inspired visualization of virtual branches as vertical lanes.
//! Each lane represents a virtual branch with its owned hunks displayed as cards.
//!
//! Layout:
//! ┌─────────────┬─────────────┬─────────────┐
//! │  Branch A   │  Branch B   │  Unassigned │
//! ├─────────────┼─────────────┼─────────────┤
//! │ [hunk 1]    │ [hunk 4]    │ [hunk 7]    │
//! │ [hunk 2]    │ [hunk 5]    │ [hunk 8]    │
//! │ [hunk 3]    │ [hunk 6]    │             │
//! │  ───────    │             │             │
//! │ [staged]    │             │             │
//! └─────────────┴─────────────┴─────────────┘

use crate::tui::app::App;
use crate::tui::theme::ThemeColors;
use crate::virtual_branch::{AgentStatus, VirtualBranch};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};

/// Lane areas for mouse hit detection
#[derive(Debug, Clone, Default)]
pub struct LaneAreas {
    /// Area for each lane column
    pub lanes: Vec<Rect>,
    /// Area for "new branch" button
    pub new_branch_btn: Option<Rect>,
}

/// Render the virtual branch lanes view
pub fn render(frame: &mut Frame, area: Rect, app: &App) -> LaneAreas {
    let colors = app.theme.colors();
    let engine = &app.theme_engine;
    let mut lane_areas = LaneAreas::default();

    // Get branches or show empty state
    let branches = match &app.vbranch_manager {
        Some(manager) => manager.list(),
        None => {
            render_empty_state(frame, area, &colors, engine);
            return lane_areas;
        }
    };

    if branches.is_empty() {
        render_no_branches(frame, area, &colors, engine);
        return lane_areas;
    }

    // Calculate column widths - split evenly
    let lane_count = branches.len();
    
    // Detect conflicts
    let conflicts = app.vbranch_manager.as_ref()
        .map(|m| m.detect_conflicts())
        .unwrap_or_default();
    
    // Create columns for each branch
    let mut constraints = branches.iter()
        .map(|_| Constraint::Percentage((95 / lane_count.max(1)) as u16))
        .collect::<Vec<_>>();
    
    // Add rightmost spacer
    constraints.push(Constraint::Percentage(5));
    
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);
    
    lane_areas.lanes = columns.iter().cloned().collect();

    // Render each lane
    for (idx, branch) in branches.iter().enumerate() {
        let is_selected = idx == app.vbranch_selected;
        let col_area = columns[idx];
        
        // Check if this branch has conflicts
        let has_conflicts = conflicts.get(&branch.id).map(|c| !c.is_empty()).unwrap_or(false);
        render_lane(
            frame,
            col_area,
            branch,
            &colors,
            engine,
            is_selected,
            if is_selected {
                Some(app.vbranch_hunk_selected)
            } else {
                None
            },
            has_conflicts,
        );
    }

    lane_areas
}

/// Render empty state (no VirtualBranchManager initialized)
fn render_empty_state(
    frame: &mut Frame,
    area: Rect,
    colors: &ThemeColors,
    engine: &crate::tui::style::ThemeEngine,
) {
    let block = Block::default()
        .title(" Virtual Branches ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(engine.get("lanes.border", colors.border()));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  No virtual branch manager initialized",
            engine.get("lanes.empty", colors.dim()),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Press 'V' to initialize virtual branches",
            colors.accent(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Virtual branches allow you to work on multiple",
            colors.dim(),
        )),
        Line::from(Span::styled(
            "  features simultaneously in the same directory.",
            colors.dim(),
        )),
    ];

    let p = Paragraph::new(lines);
    frame.render_widget(p, inner);
}

/// Render state when no branches exist
fn render_no_branches(
    frame: &mut Frame,
    area: Rect,
    colors: &ThemeColors,
    engine: &crate::tui::style::ThemeEngine,
) {
    let block = Block::default()
        .title(" Virtual Branches ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(engine.get("lanes.border", colors.border()));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  No virtual branches yet",
            engine.get("lanes.empty.title", colors.warning()),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Press 'n' to create a new virtual branch",
            colors.accent(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Each branch can own different hunks of your changes,",
            colors.dim(),
        )),
        Line::from(Span::styled(
            "  letting you commit them separately.",
            colors.dim(),
        )),
    ];

    let p = Paragraph::new(lines);
    frame.render_widget(p, inner);
}

/// Render a single lane (virtual branch column)
#[allow(clippy::too_many_arguments)]
fn render_lane(
    frame: &mut Frame,
    area: Rect,
    branch: &VirtualBranch,
    colors: &ThemeColors,
    engine: &crate::tui::style::ThemeEngine,
    is_selected: bool,
    selected_hunk: Option<usize>,
    has_conflicts: bool,
) {
    // Split: header info + hunk list + staged section
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header with branch info
            Constraint::Min(5),    // Unstaged hunks
            Constraint::Length(4), // Staged section
        ])
        .split(area);

    // Render header
    render_lane_header(frame, chunks[0], branch, colors, engine, is_selected, has_conflicts);

    // Render unstaged hunks
    render_hunk_list(
        frame,
        chunks[1],
        branch,
        colors,
        engine,
        is_selected,
        selected_hunk,
        false, // unstaged
    );

    // Render staged section
    render_staged_section(frame, chunks[2], branch, colors, engine);
}

/// Render the lane header with branch name and status
fn render_lane_header(
    frame: &mut Frame,
    area: Rect,
    branch: &VirtualBranch,
    colors: &ThemeColors,
    engine: &crate::tui::style::ThemeEngine,
    is_selected: bool,
    _has_conflicts: bool,
) {
    let border_style = if is_selected {
        engine.get("lanes.header.selected", colors.accent())
    } else {
        engine.get("lanes.header.normal", colors.border())
    };

    // Agent indicator
    let agent_indicator = match &branch.agent_session {
        Some(session) => match &session.status {
            AgentStatus::Idle => Span::styled(" 🤖", colors.dim()),
            AgentStatus::Thinking { .. } => Span::styled(" 🤔", colors.warning()),
            AgentStatus::Executing { .. } => Span::styled(" ⚡", colors.success()),
            AgentStatus::AwaitingPermission { .. } => Span::styled(" ❓", colors.error()),
            AgentStatus::Completed { .. } => Span::styled(" ✅", colors.success()),
            AgentStatus::Error { .. } => Span::styled(" ❌", colors.error()),
        },
        None => Span::raw(""),
    };

    // Conflict indicator
    let conflict_indicator = if branch.has_conflicts {
        Span::styled(" ⚠️", colors.error())
    } else {
        Span::raw("")
    };

    let title = Line::from(vec![
        Span::styled(&branch.name, border_style.add_modifier(Modifier::BOLD)),
        agent_indicator,
        conflict_indicator,
    ]);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Show branch stats
    let stats = format!(
        "{} hunks | {} staged",
        branch.owned_hunks.len(),
        branch.staged_hunks.len()
    );
    let stats_line = Paragraph::new(Span::styled(stats, colors.dim()));
    frame.render_widget(stats_line, inner);
}

/// Render the list of hunks in a lane
#[allow(clippy::too_many_arguments)]
fn render_hunk_list(
    frame: &mut Frame,
    area: Rect,
    branch: &VirtualBranch,
    colors: &ThemeColors,
    engine: &crate::tui::style::ThemeEngine,
    is_selected_lane: bool,
    selected_hunk: Option<usize>,
    _is_staged: bool,
) {
    let items: Vec<ListItem> = branch
        .owned_hunks
        .iter()
        .enumerate()
        .filter(|(_, h)| !branch.staged_hunks.contains(h)) // Exclude staged
        .enumerate()
        .map(|(display_idx, (_, hunk))| {
            let is_selected = is_selected_lane && selected_hunk == Some(display_idx);

            let prefix = if is_selected { "▶ " } else { "  " };

            // Truncate file path for display
            let file_display = if hunk.file_path.len() > 20 {
                format!("…{}", &hunk.file_path[hunk.file_path.len() - 19..])
            } else {
                hunk.file_path.clone()
            };

            let style = if is_selected {
                engine.get("lanes.hunk.selected", colors.selected())
            } else {
                engine.get("lanes.hunk.normal", colors.normal())
            };

            let content = format!(
                "{}{}:{}+{}",
                prefix, file_display, hunk.new_start, hunk.new_count
            );

            ListItem::new(content).style(style)
        })
        .collect();

    let title = if items.is_empty() {
        " Hunks (empty) "
    } else {
        " Hunks "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
        .border_style(engine.get("lanes.hunks.border", colors.border()));

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

/// Render the staged section of a lane
fn render_staged_section(
    frame: &mut Frame,
    area: Rect,
    branch: &VirtualBranch,
    colors: &ThemeColors,
    engine: &crate::tui::style::ThemeEngine,
) {
    let staged_count = branch.staged_hunks.len();

    let title_style = if staged_count > 0 {
        engine.get("lanes.staged.active", colors.success())
    } else {
        engine.get("lanes.staged.empty", colors.dim())
    };

    let title = format!(" Staged ({}) ", staged_count);

    let block = Block::default()
        .title(Span::styled(title, title_style))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(engine.get("lanes.staged.border", colors.border()));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if staged_count > 0 {
        // Show first few staged files
        let staged_files: Vec<Line> = branch
            .staged_hunks
            .iter()
            .take(2)
            .map(|h| {
                let file = if h.file_path.len() > 18 {
                    format!("…{}", &h.file_path[h.file_path.len() - 17..])
                } else {
                    h.file_path.clone()
                };
                Line::from(Span::styled(format!("  + {}", file), colors.success()))
            })
            .collect();

        let p = Paragraph::new(staged_files);
        frame.render_widget(p, inner);
    } else {
        let hint = Paragraph::new(Span::styled("  Space to stage", colors.dim()));
        frame.render_widget(hint, inner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lane_areas_default() {
        let areas = LaneAreas::default();
        assert!(areas.lanes.is_empty());
        assert!(areas.new_branch_btn.is_none());
    }
}
