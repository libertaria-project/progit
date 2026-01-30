// SPDX-License-Identifier: EUPL-1.2
// Copyright (c) 2025 Markus Maiwald

//! Code review widget
//!
//! Shows diff with line-level comments. Press 'c' to add a comment.

use crate::review::{Review, ReviewComment, ReviewStorage};
use crate::tui::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use std::collections::HashMap;

/// Review widget state
pub struct ReviewState {
    /// Current review
    pub review: Option<Review>,

    /// Comments grouped by file and line
    pub comments_by_line: HashMap<String, HashMap<usize, Vec<ReviewComment>>>,

    /// Currently selected line (for adding comment)
    pub selected_line: usize,

    /// Currently viewing file
    pub current_file: String,

    /// Diff lines
    pub diff_lines: Vec<DiffLine>,

    /// Scroll offset
    pub scroll_offset: usize,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub line_number: usize,
    pub content: String,
    pub line_type: DiffLineType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineType {
    Context,
    Added,
    Removed,
    Header,
}

impl Default for ReviewState {
    fn default() -> Self {
        Self {
            review: None,
            comments_by_line: HashMap::new(),
            selected_line: 1,
            current_file: String::new(),
            diff_lines: Vec::new(),
            scroll_offset: 0,
        }
    }
}

impl ReviewState {
    /// Create new review state from diff
    pub fn from_diff(file_path: String, diff_text: String, commit_sha: String) -> Self {
        let diff_lines = parse_diff(&diff_text);

        Self {
            review: None,
            comments_by_line: HashMap::new(),
            selected_line: 1,
            current_file: file_path,
            diff_lines,
            scroll_offset: 0,
        }
    }

    /// Load existing review
    pub fn load_review(&mut self, storage: &ReviewStorage, review_id: &str) -> anyhow::Result<()> {
        let review = storage.load(review_id)?;

        // Group comments by file and line
        let mut grouped: HashMap<String, HashMap<usize, Vec<ReviewComment>>> = HashMap::new();
        for comment in &review.comments {
            grouped
                .entry(comment.file_path.clone())
                .or_default()
                .entry(comment.line_number)
                .or_default()
                .push(comment.clone());
        }

        self.comments_by_line = grouped;
        self.review = Some(review);

        Ok(())
    }

    /// Add a comment to the current line
    pub fn add_comment(&mut self, text: String, author: String) -> ReviewComment {
        let comment = ReviewComment {
            id: format!("comment-{}", uuid::Uuid::new_v4()),
            file_path: self.current_file.clone(),
            line_number: self.selected_line,
            commit_sha: self.review.as_ref().map(|r| r.commit_sha.clone()).unwrap_or_default(),
            text,
            author,
            created_at: chrono::Utc::now().to_rfc3339(),
            resolved: false,
            replies: vec![],
        };

        // Add to local state
        self.comments_by_line
            .entry(self.current_file.clone())
            .or_default()
            .entry(self.selected_line)
            .or_default()
            .push(comment.clone());

        comment
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        if self.selected_line > 1 {
            self.selected_line -= 1;

            // Adjust scroll if needed
            if self.selected_line < self.scroll_offset + 3 && self.scroll_offset > 0 {
                self.scroll_offset -= 1;
            }
        }
    }

    /// Move selection down
    pub fn move_down(&mut self, visible_lines: usize) {
        if self.selected_line < self.diff_lines.len() {
            self.selected_line += 1;

            // Adjust scroll if needed
            if self.selected_line > self.scroll_offset + visible_lines - 3 {
                self.scroll_offset += 1;
            }
        }
    }
}

/// Parse diff text into structured lines
fn parse_diff(diff_text: &str) -> Vec<DiffLine> {
    let mut lines = Vec::new();
    let mut line_number = 0;

    for line in diff_text.lines() {
        line_number += 1;

        let line_type = if line.starts_with("+++") || line.starts_with("---") || line.starts_with("@@") {
            DiffLineType::Header
        } else if line.starts_with('+') {
            DiffLineType::Added
        } else if line.starts_with('-') {
            DiffLineType::Removed
        } else {
            DiffLineType::Context
        };

        lines.push(DiffLine {
            line_number,
            content: line.to_string(),
            line_type,
        });
    }

    lines
}

/// Render the review widget
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    if app.review_state.is_none() {
        let placeholder = Paragraph::new("No review loaded. Use :review <file> to start.")
            .block(Block::default().borders(Borders::ALL).title("Code Review"));
        frame.render_widget(placeholder, area);
        return;
    }

    let state = app.review_state.as_ref().unwrap();

    // Split into diff view (left) and comments (right)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    render_diff_view(frame, app, state, chunks[0]);
    render_comments_sidebar(frame, app, state, chunks[1]);
}

/// Render diff view with inline comment indicators
fn render_diff_view(frame: &mut Frame, app: &App, state: &ReviewState, area: Rect) {
    let colors = app.theme.colors();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Review: {}", state.current_file));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible_lines = inner.height as usize;
    let start = state.scroll_offset;
    let end = (start + visible_lines).min(state.diff_lines.len());

    let items: Vec<ListItem> = state.diff_lines[start..end]
        .iter()
        .map(|diff_line| {
            let is_selected = diff_line.line_number == state.selected_line;
            let has_comments = state
                .comments_by_line
                .get(&state.current_file)
                .and_then(|lines: &HashMap<usize, Vec<ReviewComment>>| lines.get(&diff_line.line_number))
                .map(|comments: &Vec<ReviewComment>| !comments.is_empty())
                .unwrap_or(false);

            let mut spans = vec![];

            // Line number
            let line_num_str = format!("{:4} ", diff_line.line_number);
            spans.push(Span::styled(
                line_num_str,
                Style::default().fg(colors.accent_dim),
            ));

            // Comment indicator
            if has_comments {
                spans.push(Span::styled("💬 ", Style::default().fg(colors.accent)));
            } else {
                spans.push(Span::raw("   "));
            }

            // Line content with appropriate coloring
            let (content_style, prefix) = match diff_line.line_type {
                DiffLineType::Added => (
                    Style::default()
                        .fg(colors.success)
                        .add_modifier(Modifier::BOLD),
                    "+",
                ),
                DiffLineType::Removed => (
                    Style::default()
                        .fg(colors.error)
                        .add_modifier(Modifier::BOLD),
                    "-",
                ),
                DiffLineType::Header => (
                    Style::default()
                        .fg(colors.accent)
                        .add_modifier(Modifier::BOLD),
                    "@",
                ),
                DiffLineType::Context => (Style::default().fg(colors.fg), " "),
            };

            spans.push(Span::styled(
                format!("{}{}", prefix, &diff_line.content),
                content_style,
            ));

            // Highlight selection
            let mut item = ListItem::new(Line::from(spans));
            if is_selected {
                item = item.style(Style::default().bg(colors.selected_bg));
            }

            item
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}

/// Render comments sidebar
fn render_comments_sidebar(frame: &mut Frame, app: &App, state: &ReviewState, area: Rect) {
    let colors = app.theme.colors();

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Comments");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Get comments for selected line
    let comments = state
        .comments_by_line
        .get(&state.current_file)
        .and_then(|lines: &HashMap<usize, Vec<ReviewComment>>| lines.get(&state.selected_line));

    if let Some(comments) = comments {
        let mut lines = Vec::new();

        for comment in comments {
            lines.push(Line::from(vec![
                Span::styled(&comment.author, Style::default().fg(colors.accent).add_modifier(Modifier::BOLD)),
                Span::raw(" • "),
                Span::styled(
                    &comment.created_at[..10], // Just the date
                    Style::default().fg(colors.accent_dim),
                ),
            ]));

            lines.push(Line::from(Span::styled(
                &comment.text,
                Style::default().fg(colors.fg),
            )));

            lines.push(Line::from(""));
        }

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true });
        frame.render_widget(paragraph, inner);
    } else {
        let help = Paragraph::new(vec![
            Line::from(Span::styled(
                "No comments on this line.",
                Style::default().fg(colors.accent_dim),
            )),
            Line::from(""),
            Line::from(Span::raw("Press 'c' to add a comment.")),
        ])
        .wrap(Wrap { trim: true });

        frame.render_widget(help, inner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_diff() {
        let diff = r#"@@ -1,3 +1,4 @@
 fn main() {
-    println!("old");
+    println!("new");
+    println!("added");
 }
"#;

        let lines = parse_diff(diff);
        assert_eq!(lines.len(), 6);
        assert_eq!(lines[0].line_type, DiffLineType::Header);
        assert_eq!(lines[2].line_type, DiffLineType::Removed);
        assert_eq!(lines[3].line_type, DiffLineType::Added);
    }
}
