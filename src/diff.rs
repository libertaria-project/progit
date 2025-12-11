// SPDX-License-Identifier: EUPL-1.2
// Copyright (c) 2025 Markus Maiwald

//! Advanced Diff Viewer
//!
//! Parses and renders git diffs with syntax highlighting support (via style).

use anyhow::{Result, Context};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, StatefulWidget, Widget},
};
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub enum DiffLineType {
    Context,
    Add,
    Delete,
    Header,     // Meta headers like "diff --git..."
    HunkHeader, // @@ ... @@
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub content: String,
    pub line_type: DiffLineType,
}

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub path: String,
    pub is_binary: bool,
    pub lines: Vec<DiffLine>,
    pub collapsed: bool,
    pub additions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone)]
pub struct DiffState {
    pub files: Vec<FileDiff>,
    pub selected_file: usize,
    pub scroll: u16,
    pub reference: String,
}

impl DiffState {
    pub fn new(reference: String) -> Self {
        Self {
            files: Vec::new(),
            selected_file: 0,
            scroll: 0,
            reference,
        }
    }

    pub fn load(&mut self) -> Result<()> {
        let mut cmd = Command::new("git");
        cmd.arg("diff");
        
        // Handle argument parsing (e.g. if it has spaces or is "HEAD~1")
        // For now, treat reference as raw args. 
        // If reference is empty, it diffs working tree.
        if !self.reference.is_empty() {
             for part in self.reference.split_whitespace() {
                 cmd.arg(part);
             }
        }

        let output = cmd.output().context("Failed to run git diff")?;
        let content = String::from_utf8_lossy(&output.stdout);
        self.parse(&content);
        Ok(())
    }

    fn parse(&mut self, content: &str) {
        self.files.clear();
        let mut current_file: Option<FileDiff> = None;

        for line in content.lines() {
            if line.starts_with("diff --git") {
                if let Some(f) = current_file.take() {
                    self.files.push(f);
                }
                // Parse "diff --git a/src/main.rs b/src/main.rs"
                // Or "diff --git a/file b/file"
                let parts: Vec<&str> = line.split_whitespace().collect();
                // Usually file path is at the end. We take the last part and strip b/
                let raw_path = parts.last().unwrap_or(&"unknown");
                let path = if raw_path.starts_with("b/") {
                    &raw_path[2..]
                } else {
                    raw_path
                }
                .to_string();

                current_file = Some(FileDiff {
                    path,
                    is_binary: false,
                    lines: Vec::new(),
                    collapsed: false,
                    additions: 0,
                    deletions: 0,
                });
            } else if let Some(ref mut file) = current_file {
                 if line.starts_with("index ") || line.starts_with("new file") || line.starts_with("deleted file") {
                     // Meta info, skip for display cleanliness? 
                     // Or add as Header
                     continue;
                 }
                 if line.starts_with("--- ") || line.starts_with("+++ ") {
                     continue;
                 }
                 if line.starts_with("Binary files") {
                     file.is_binary = true;
                     file.lines.push(DiffLine { content: line.to_string(), line_type: DiffLineType::Header });
                     continue;
                 }

                let line_type = if line.starts_with("@@") {
                    DiffLineType::HunkHeader
                } else if line.starts_with('+') {
                    file.additions += 1;
                    DiffLineType::Add
                } else if line.starts_with('-') {
                    file.deletions += 1;
                    DiffLineType::Delete
                } else {
                    DiffLineType::Context
                };

                file.lines.push(DiffLine {
                    content: line.to_string(),
                    line_type,
                });
            }
        }
        if let Some(f) = current_file {
            self.files.push(f);
        }
    }
}

// Widget Implementation via function for now (simpler integration in tui.rs)
pub fn render_diff(f: &mut ratatui::Frame, area: Rect, state: &DiffState) {
    use ratatui::widgets::{List, ListItem, Paragraph};
    
    if state.files.is_empty() {
        f.render_widget(
            Paragraph::new("No changes detected.")
                .block(Block::default().borders(Borders::ALL).title(" Diff ")),
            area
        );
        return;
    }

    // Since we want a foldable view, let's construct a list of items where some are headers and some are content
    // But dealing with massive scroll lists is tricky.
    // Simpler: Just render the *selected* file's diff?
    // Or render a list of files on the left, and diff on right?
    // "Unified" View: Just big scrollable list of all files.
    
    // Let's do: List of content lines.
    let mut items: Vec<ListItem> = Vec::new();
    
    for (idx, file) in state.files.iter().enumerate() {
        // File Header
        let is_selected = idx == state.selected_file;
        let header_style = if is_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
        };
        
        let icon = if file.collapsed { "▶" } else { "▼" };
        let stats = format!("+{} -{}", file.additions, file.deletions);
        let header_content = format!("{} {}  ({})", icon, file.path, stats);
        
        items.push(ListItem::new(Line::from(vec![
            Span::styled(header_content, header_style)
        ])));

        if !file.collapsed {
            if file.is_binary {
                items.push(ListItem::new(Line::from(Span::raw("  <Binary File Details Hidden>"))));
            } else {
                for line in &file.lines {
                    let (fg, bg) = match line.line_type {
                        DiffLineType::Add => (Color::Green, Color::Black),
                        DiffLineType::Delete => (Color::Red, Color::Black),
                        DiffLineType::HunkHeader => (Color::Cyan, Color::Black),
                        _ => (Color::Reset, Color::Reset),
                    };
                    
                    // Highlight full background would be nicer but ratatui ListItems bg covers whole width?
                    // Let's try to just color text for start, maybe bg for +/- char.
                    
                    let mut spans = Vec::new();
                    // Colored prefix (+/-)
                    let prefix = if line.content.len() > 0 { &line.content[0..1] } else { " " };
                    let rest = if line.content.len() > 1 { &line.content[1..] } else { "" };
                    
                    if line.line_type == DiffLineType::Add {
                        spans.push(Span::styled(prefix, Style::default().bg(Color::Green).fg(Color::Black)));
                        spans.push(Span::styled(rest, Style::default().fg(Color::Green)));
                    } else if line.line_type == DiffLineType::Delete {
                        spans.push(Span::styled(prefix, Style::default().bg(Color::Red).fg(Color::Black)));
                        spans.push(Span::styled(rest, Style::default().fg(Color::Red)));
                    } else if line.line_type == DiffLineType::HunkHeader {
                         spans.push(Span::styled(&line.content, Style::default().fg(Color::Cyan)));
                    } else {
                         spans.push(Span::raw(&line.content));
                    }
                    
                    items.push(ListItem::new(Line::from(spans)));
                }
            }
        }
    }
    
    // We need stateful list to handle scrolling if we use List widget
    // But we are managing selection via `state.selected_file` which is file index, not line index.
    // This mismatch makes a single List hard.
    
    // Better Approach: 
    // Two Panes: Left = File List, Right = Diff Content
    // This is "Advanced".
    
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([ratatui::layout::Constraint::Percentage(30), ratatui::layout::Constraint::Percentage(70)])
        .split(area);
        
    // Left: File List
    let files: Vec<ListItem> = state.files.iter().enumerate().map(|(i, f)| {
        let style = if i == state.selected_file {
            Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        ListItem::new(format!("{} (+{}/-{})", f.path, f.additions, f.deletions)).style(style)
    }).collect();
    
    let file_list = List::new(files)
        .block(Block::default().borders(Borders::ALL).title(" Files "));
        
    f.render_widget(file_list, chunks[0]);
    
    // Right: Content
    if let Some(file) = state.files.get(state.selected_file) {
        let content_items: Vec<ListItem> = file.lines.iter().map(|line| {
             let mut spans = Vec::new();
             if line.line_type == DiffLineType::Add {
                // Green
                spans.push(Span::styled(&line.content, Style::default().fg(Color::Green)));
             } else if line.line_type == DiffLineType::Delete {
                // Red
                spans.push(Span::styled(&line.content, Style::default().fg(Color::Red)));
             } else if line.line_type == DiffLineType::HunkHeader {
                spans.push(Span::styled(&line.content, Style::default().fg(Color::Cyan)));
             } else {
                spans.push(Span::raw(&line.content));
             }
             ListItem::new(Line::from(spans))
        }).collect();
        
        let content_list = List::new(content_items)
            .block(Block::default().borders(Borders::ALL).title(format!(" Diff: {} ", file.path)));
            
        // We need a Scroll offset for the content
        // We'll trust the user to keybind scrolling.
        // We need `ListState` to scroll.
        // We'll just start at `state.scroll`
        
        // Ratatui List doesn't take raw offset easily without State.
        // We'll create a transient ListState.
        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(state.scroll as usize)); // Using select as scroll anchor? No, ListState uses offset.
        // select(idx) scrolls to make regular items visible.
        // offset is internal.
        
        // Actually for pure content viewing, Paragraph is better if we just dump lines?
        // But Paragraph doesn't do syntax coloring easily line-by-line as generic as List.
        // List is fine.
        
        // Hack: Scroll behavior.
        // `f.render_widget` renders. `render_stateful_widget`.
        // If we want to scroll, we should adjust `state.scroll` and pass it.
        // BUT `List` widget tracks *selected item*.
        // We want to scroll the *view*.
        // If we use `select(state.scroll)`, it highlights that line. That's fine! 
        // It acts as a cursor in the diff.
        
        f.render_stateful_widget(content_list, chunks[1], &mut list_state);
    }
}
