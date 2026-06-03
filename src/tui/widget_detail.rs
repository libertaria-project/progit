//! Widget Detail - Issue detail pane
//!
//! Full view of a single issue for viewing/editing.

use crate::issue::{Effort, Issue, Status};
use crate::tui::markdown::render_markdown;
use crate::tui::theme::ThemeColors;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Which field is being edited
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EditField {
    #[default]
    Title,
    Description,
    Status,
    Effort,
    Assignee,
    Tags,
    DueDate,
    StartedDate,
    CompletedDate,
}

impl EditField {
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => EditField::Title,
            1 => EditField::Description,
            2 => EditField::Status,
            3 => EditField::Effort,
            4 => EditField::Assignee,
            5 => EditField::Tags,
            6 => EditField::DueDate,
            7 => EditField::StartedDate,
            _ => EditField::CompletedDate,
        }
    }

    pub fn next(self) -> Self {
        match self {
            EditField::Title => EditField::Description,
            EditField::Description => EditField::Status,
            EditField::Status => EditField::Effort,
            EditField::Effort => EditField::Assignee,
            EditField::Assignee => EditField::Tags,
            EditField::Tags => EditField::DueDate,
            EditField::DueDate => EditField::StartedDate,
            EditField::StartedDate => EditField::CompletedDate,
            EditField::CompletedDate => EditField::Title,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            EditField::Title => EditField::CompletedDate,
            EditField::Description => EditField::Title,
            EditField::Status => EditField::Description,
            EditField::Effort => EditField::Status,
            EditField::Assignee => EditField::Effort,
            EditField::Tags => EditField::Assignee,
            EditField::DueDate => EditField::Tags,
            EditField::StartedDate => EditField::DueDate,
            EditField::CompletedDate => EditField::StartedDate,
        }
    }
}

/// Render the issue detail pane as an overlay
/// Returns: (detail_area, close_button_area)
pub fn render(
    frame: &mut Frame,
    issue: &Issue,
    edit_field: EditField,
    edit_buffer: &str,
    colors: &ThemeColors,
) -> (Rect, Rect) {
    let area = frame.area();

    // Center the detail pane (80% width, 80% height)
    let width = (area.width * 80 / 100).min(80);
    let height = (area.height * 80 / 100).min(30);
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;

    let detail_area = Rect::new(x, y, width, height);

    // Close button area [X] - top right corner
    let close_btn = Rect::new(x + width - 4, y, 3, 1);

    // Clear the area behind the popup
    frame.render_widget(Clear, detail_area);

    // Main block with [X] close button in title
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" 📝 Issue: ", colors.accent()),
            Span::styled(issue.short_id(), colors.dim()),
            Span::raw(" "),
        ]))
        .title_bottom(Line::from(vec![
            Span::styled(" Tab", colors.accent()),
            Span::raw(":fields "),
            Span::styled("Space", colors.accent()),
            Span::raw(":cycle "),
            Span::styled("Enter", colors.accent()),
            Span::raw(":edit "),
            Span::styled("Esc", colors.accent()),
            Span::raw(":close "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(colors.accent());

    let inner = block.inner(detail_area);
    frame.render_widget(block, detail_area);

    // Render [X] close button
    let close_widget =
        Paragraph::new(Span::styled("[X]", colors.warning())).alignment(Alignment::Center);
    frame.render_widget(close_widget, close_btn);

    // Split into rows for each field
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(5), // Description
            Constraint::Length(3), // Status + Effort (side by side)
            Constraint::Length(3), // Assignee
            Constraint::Length(3), // Tags
            Constraint::Length(3), // Due / Started / Completed
            Constraint::Min(1),    // Spacer
        ])
        .split(inner);

    // Title field
    render_field(
        frame,
        chunks[0],
        "Title",
        if edit_field == EditField::Title {
            edit_buffer
        } else {
            &issue.title
        },
        edit_field == EditField::Title,
        colors,
    );

    // Description field - render as markdown when not editing
    if edit_field == EditField::Description {
        render_multiline_field(frame, chunks[1], "Description", edit_buffer, true, colors);
    } else {
        render_markdown_field(
            frame,
            chunks[1],
            "Description",
            &issue.description,
            false,
            colors,
        );
    }

    // Status + Effort row
    let status_effort = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);

    render_status_field(
        frame,
        status_effort[0],
        "Status",
        if edit_field == EditField::Status {
            edit_buffer
        } else {
            issue.status.as_str()
        },
        issue.status,
        edit_field == EditField::Status,
        colors,
    );

    let effort_str = format!("{}", issue.effort as u8);
    render_effort_field(
        frame,
        status_effort[1],
        "Effort",
        if edit_field == EditField::Effort {
            edit_buffer
        } else {
            &effort_str
        },
        issue.effort,
        edit_field == EditField::Effort,
        colors,
    );

    // Assignee field
    let assignee_display = issue.assignee.as_deref().unwrap_or("(none)");
    render_field(
        frame,
        chunks[3],
        "Assignee",
        if edit_field == EditField::Assignee {
            edit_buffer
        } else {
            assignee_display
        },
        edit_field == EditField::Assignee,
        colors,
    );

    // Tags field
    let tags_display = if issue.tags.is_empty() {
        "(none)".to_string()
    } else {
        issue.tags.join(", ")
    };
    render_field(
        frame,
        chunks[4],
        "Tags",
        if edit_field == EditField::Tags {
            edit_buffer
        } else {
            &tags_display
        },
        edit_field == EditField::Tags,
        colors,
    );

    // Time tracking row (Due / Started / Completed)
    let time_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(chunks[5]);

    let due_display = issue
        .due
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or("(none)".to_string());
    render_field(
        frame,
        time_row[0],
        "⏰ Due",
        if edit_field == EditField::DueDate {
            edit_buffer
        } else {
            &due_display
        },
        edit_field == EditField::DueDate,
        colors,
    );

    let started_display = issue
        .started
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or("(none)".to_string());
    render_field(
        frame,
        time_row[1],
        "▶ Started",
        if edit_field == EditField::StartedDate {
            edit_buffer
        } else {
            &started_display
        },
        edit_field == EditField::StartedDate,
        colors,
    );

    let completed_display = issue
        .completed
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or("(none)".to_string());
    render_field(
        frame,
        time_row[2],
        "✓ Completed",
        if edit_field == EditField::CompletedDate {
            edit_buffer
        } else {
            &completed_display
        },
        edit_field == EditField::CompletedDate,
        colors,
    );

    // Return areas for mouse handling: detail_area, close_btn, and field areas
    // Field areas: [title, desc, status, effort, assignee, tags, due, started, completed]
    (detail_area, close_btn)
}

fn render_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    is_selected: bool,
    colors: &ThemeColors,
) {
    let style = if is_selected {
        colors.selected()
    } else {
        colors.normal()
    };

    let border_style = if is_selected {
        colors.accent()
    } else {
        colors.border()
    };

    let cursor = if is_selected { "█" } else { "" };

    let content = Paragraph::new(Line::from(vec![
        Span::styled(value, style),
        Span::styled(cursor, colors.accent()),
    ]))
    .block(
        Block::default()
            .title(Span::styled(format!(" {} ", label), colors.dim()))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style),
    );

    frame.render_widget(content, area);
}

fn render_multiline_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    is_selected: bool,
    colors: &ThemeColors,
) {
    let style = if is_selected {
        colors.selected()
    } else {
        colors.normal()
    };

    let border_style = if is_selected {
        colors.accent()
    } else {
        colors.border()
    };

    let cursor = if is_selected { "█" } else { "" };

    let content = Paragraph::new(format!("{}{}", value, cursor))
        .style(style)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title(Span::styled(format!(" {} ", label), colors.dim()))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style),
        );

    frame.render_widget(content, area);
}

/// Render a field with markdown formatting
fn render_markdown_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    is_selected: bool,
    colors: &ThemeColors,
) {
    let border_style = if is_selected {
        colors.accent()
    } else {
        colors.border()
    };

    // Parse and render markdown
    let base_style = colors.normal();
    let markdown_text = render_markdown(value, base_style);

    let content = Paragraph::new(markdown_text)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title(Span::styled(format!(" {} ", label), colors.dim()))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(border_style),
        );

    frame.render_widget(content, area);
}

fn render_status_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    _value: &str,
    status: Status,
    is_selected: bool,
    colors: &ThemeColors,
) {
    let border_style = if is_selected {
        colors.accent()
    } else {
        colors.border()
    };

    // Show all status options with current highlighted
    let options = [Status::Backlog, Status::InProgress, Status::Done];
    let spans: Vec<Span> = options
        .iter()
        .map(|s| {
            let style = if *s == status {
                colors.accent().add_modifier(Modifier::BOLD)
            } else {
                colors.dim()
            };
            let prefix = if *s == status { "●" } else { "○" };
            Span::styled(format!(" {}{} ", prefix, s.as_str()), style)
        })
        .collect();

    let content = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .title(Span::styled(format!(" {} ", label), colors.dim()))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style),
    );

    frame.render_widget(content, area);
}

fn render_effort_field(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    _value: &str,
    effort: Effort,
    is_selected: bool,
    colors: &ThemeColors,
) {
    let border_style = if is_selected {
        colors.accent()
    } else {
        colors.border()
    };

    // Show effort options
    let options = [
        Effort::Trivial,
        Effort::Small,
        Effort::Medium,
        Effort::Large,
        Effort::XLarge,
        Effort::Epic,
    ];
    let spans: Vec<Span> = options
        .iter()
        .map(|e| {
            let style = if *e == effort {
                colors.accent().add_modifier(Modifier::BOLD)
            } else {
                colors.dim()
            };
            Span::styled(format!(" {} ", *e as u8), style)
        })
        .collect();

    let content = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .title(Span::styled(format!(" {} ", label), colors.dim()))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style),
    );

    frame.render_widget(content, area);
}
