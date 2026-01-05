//! Widget MR Create - Merge Request creation form
//!
//! Floating form for creating merge requests.

use crate::mr::MergeRequest;
use crate::tui::theme::ThemeColors;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, BorderType, Clear, Paragraph, Wrap},
    Frame,
};

/// Render the MR creation form as an overlay
pub fn render(frame: &mut Frame, mr: &MergeRequest, current_field: usize, edit_buffer: &str, colors: &ThemeColors) -> Rect {
    let area = frame.area();

    // Center the form (70% width, 60% height)
    let width = (area.width * 70 / 100).min(70);
    let height = (area.height * 60 / 100).min(20);
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;

    let form_area = Rect::new(x, y, width, height);

    // Clear the area behind the popup
    frame.render_widget(Clear, form_area);

    // Main block
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" 🔀 Create Merge Request ", colors.accent()),
        ]))
        .title_bottom(Line::from(vec![
            Span::styled(" Tab", colors.accent()),
            Span::raw(":next field "),
            Span::styled("Enter", colors.accent()),
            Span::raw(":submit "),
            Span::styled("Esc", colors.accent()),
            Span::raw(":cancel "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(colors.accent());

    let inner = block.inner(form_area);
    frame.render_widget(block, form_area);

    // Split into fields
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Source branch
            Constraint::Length(3), // Target branch
            Constraint::Length(3), // Title
            Constraint::Min(5),    // Description (multi-line)
            Constraint::Length(1), // Spacer
        ])
        .split(inner);

    // Source branch (readonly, shown for context)
    let source_field = Paragraph::new(Line::from(vec![
        Span::styled(&mr.source_branch, colors.success().add_modifier(Modifier::BOLD)),
    ]))
    .block(
        Block::default()
            .title(Span::styled(" Source Branch ", colors.dim()))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(colors.border()),
    );
    frame.render_widget(source_field, chunks[0]);

    // Target branch (field 0)
    let target_selected = current_field == 0;
    let target_cursor = if target_selected { "█" } else { "" };
    let target_field = Paragraph::new(Line::from(vec![
        Span::styled(if target_selected { edit_buffer } else { &mr.target_branch }, colors.normal()),
        Span::styled(target_cursor, colors.accent()),
    ]))
    .block(
        Block::default()
            .title(Span::styled(" Target Branch ", colors.dim()))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if target_selected { colors.accent() } else { colors.border() }),
    );
    frame.render_widget(target_field, chunks[1]);

    // Title (field 1)
    let title_selected = current_field == 1;
    let title_cursor = if title_selected { "█" } else { "" };
    let title_field = Paragraph::new(Line::from(vec![
        Span::styled(if title_selected { edit_buffer } else { &mr.title }, colors.normal()),
        Span::styled(title_cursor, colors.accent()),
    ]))
    .block(
        Block::default()
            .title(Span::styled(" Title ", colors.dim()))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if title_selected { colors.accent() } else { colors.border() }),
    );
    frame.render_widget(title_field, chunks[2]);

    // Description (field 2, multi-line)
    let desc_selected = current_field == 2;
    let desc_cursor = if desc_selected { "█" } else { "" };
    let desc_field = Paragraph::new(format!(
        "{}{}",
        if desc_selected { edit_buffer } else { &mr.description },
        desc_cursor
    ))
    .style(colors.normal())
    .wrap(Wrap { trim: false })
    .block(
        Block::default()
            .title(Span::styled(" Description ", colors.dim()))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if desc_selected { colors.accent() } else { colors.border() }),
    );
    frame.render_widget(desc_field, chunks[3]);

    form_area
}
