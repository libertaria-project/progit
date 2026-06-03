// SPDX-License-Identifier: EUPL-1.2
// Copyright (c) 2025 Markus Maiwald

//! Agent Menu Widget
//!
//! [ARCH] Modal menu for selecting AI agent actions on virtual branches.
//! Provides curated prompts for common development tasks.

use crate::tui::app::App;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};

/// Available agent actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentAction {
    ExplainHunks,
    GenerateTests,
    RefactorCode,
    AddDocumentation,
    FindBugs,
    OptimizePerformance,
    GenerateCommitMessage,
}

impl AgentAction {
    /// Get all available actions
    pub fn all() -> Vec<Self> {
        vec![
            Self::ExplainHunks,
            Self::GenerateTests,
            Self::RefactorCode,
            Self::AddDocumentation,
            Self::FindBugs,
            Self::OptimizePerformance,
            Self::GenerateCommitMessage,
        ]
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ExplainHunks => "📖 Explain Changes",
            Self::GenerateTests => "🧪 Generate Tests",
            Self::RefactorCode => "♻️  Refactor Code",
            Self::AddDocumentation => "📝 Add Documentation",
            Self::FindBugs => "🐛 Find Bugs",
            Self::OptimizePerformance => "⚡ Optimize Performance",
            Self::GenerateCommitMessage => "💬 Generate Commit Message",
        }
    }

    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            Self::ExplainHunks => "Ask the agent to explain what changed and why",
            Self::GenerateTests => "Generate unit tests for the modified code",
            Self::RefactorCode => "Improve code structure and readability",
            Self::AddDocumentation => "Add docstrings and comments",
            Self::FindBugs => "Analyze code for potential bugs and edge cases",
            Self::OptimizePerformance => "Suggest performance improvements",
            Self::GenerateCommitMessage => "Generate a descriptive commit message",
        }
    }

    /// Get the prompt template for this action
    pub fn prompt_template(&self) -> &'static str {
        match self {
            Self::ExplainHunks => {
                "Analyze the code changes shown above. Explain:\n\
                 1. What functionality was added, modified, or removed\n\
                 2. Why these changes might have been made\n\
                 3. Any potential side effects or implications\n\n\
                 Provide a clear, technical explanation suitable for code review."
            }
            Self::GenerateTests => {
                "Generate comprehensive unit tests for the modified code above.\n\
                 - Cover edge cases and error conditions\n\
                 - Use the project's existing test framework\n\
                 - Include test descriptions and assertions\n\n\
                 Return the test code as a complete, runnable test file."
            }
            Self::RefactorCode => {
                "Analyze the code above and suggest refactoring improvements:\n\
                 - Extract duplicated code into reusable functions\n\
                 - Improve naming and code clarity\n\
                 - Apply SOLID principles\n\
                 - Maintain existing functionality\n\n\
                 Return a Unified Diff with the refactored code."
            }
            Self::AddDocumentation => {
                "Add comprehensive documentation to the code above:\n\
                 - Add docstrings to all public functions and structs\n\
                 - Explain parameters, return values, and errors\n\
                 - Add inline comments for complex logic\n\
                 - Follow the project's documentation style\n\n\
                 Return a Unified Diff with the documented code."
            }
            Self::FindBugs => {
                "Analyze the code above for potential bugs and issues:\n\
                 - Check for null pointer dereferences, race conditions, memory leaks\n\
                 - Identify edge cases that aren't handled\n\
                 - Look for logic errors and off-by-one errors\n\
                 - Suggest fixes for any issues found\n\n\
                 Provide a detailed bug report with severity and suggested fixes."
            }
            Self::OptimizePerformance => {
                "Analyze the code above for performance improvements:\n\
                 - Identify algorithmic inefficiencies (O(n²) → O(n log n), etc.)\n\
                 - Suggest better data structures\n\
                 - Find unnecessary allocations or copies\n\
                 - Recommend caching or memoization opportunities\n\n\
                 Return a Unified Diff with optimized code and explain the improvements."
            }
            Self::GenerateCommitMessage => {
                "Generate a descriptive commit message for the changes above.\n\
                 Follow Conventional Commits format:\n\
                 - type(scope): subject (50 chars or less)\n\
                 - Blank line\n\
                 - Body explaining what and why (72 char lines)\n\
                 - Footer with breaking changes if any\n\n\
                 Be specific and technical. Focus on the 'why', not just the 'what'."
            }
        }
    }

    /// Get system prompt for this action
    pub fn system_prompt(&self) -> &'static str {
        match self {
            Self::ExplainHunks | Self::FindBugs | Self::GenerateCommitMessage => {
                "You are a senior software engineer performing code review. \
                 Be thorough, precise, and constructive. Focus on technical accuracy."
            }
            Self::GenerateTests => {
                "You are a senior QA engineer specializing in test-driven development. \
                 Write comprehensive, maintainable tests that cover edge cases."
            }
            Self::RefactorCode | Self::AddDocumentation | Self::OptimizePerformance => {
                "You are a senior software engineer. Return ONLY valid Unified Diff format. \
                 Verify all diff headers match the actual file paths. \
                 Preserve existing functionality while improving code quality."
            }
        }
    }
}

/// Render the agent menu modal
pub fn render(frame: &mut Frame, app: &App, selected: usize) {
    let colors = app.theme.colors();

    // Create centered modal (60% width, 70% height)
    let area = centered_rect(60, 70, frame.area());

    // Clear background
    let block = Block::default()
        .title(" 🤖 AI Agent Actions ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split: actions list (60%) | description (40%)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(inner);

    // Render action list
    let actions = AgentAction::all();
    let items: Vec<ListItem> = actions
        .iter()
        .enumerate()
        .map(|(idx, action)| {
            let prefix = if idx == selected { "▶ " } else { "  " };
            let style = if idx == selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(format!("{}{}", prefix, action.display_name())).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Select Action ")
            .borders(Borders::ALL)
            .border_style(colors.border()),
    );

    frame.render_widget(list, chunks[0]);

    // Render description for selected action
    if let Some(action) = actions.get(selected) {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                action.description(),
                Style::default().fg(Color::Gray),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press ", colors.dim()),
                Span::styled(
                    "Enter",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to execute | ", colors.dim()),
                Span::styled(
                    "Esc",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to cancel", colors.dim()),
            ]),
        ];

        let para = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" Description ")
                    .borders(Borders::ALL)
                    .border_style(colors.border()),
            )
            .alignment(Alignment::Left);

        frame.render_widget(para, chunks[1]);
    }
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
