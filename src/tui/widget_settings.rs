//! Widget Settings - Settings pane overlay
//!
//! Configuration panel for theme, sync, and other options.

use crate::tui::app::App;
use crate::tui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Settings field being edited
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsField {
    #[default]
    Theme,
    // Future: SyncProvider, CachePath, etc.
}

impl SettingsField {
    pub fn next(self) -> Self {
        // For now, only one field
        Self::Theme
    }

    pub fn prev(self) -> Self {
        Self::Theme
    }
}

/// Render the settings pane (centered overlay)
pub fn render(frame: &mut Frame, app: &App, selected_field: usize) -> Rect {
    let colors = app.theme.colors();
    let size = frame.area();

    // Center the settings pane (60% width, 50% height)
    let area = centered_rect(size, 60, 50);

    // Clear background
    frame.render_widget(Clear, area);

    // Build content
    let mut lines = Vec::new();

    // Theme selector
    let themes = ["Nord", "Gruvbox", "Dracula", "Cyberpunk", "Vibe"];
    let current_theme_idx = match app.theme {
        Theme::Nord => 0,
        Theme::Gruvbox => 1,
        Theme::Dracula => 2,
        Theme::Cyberpunk => 3,
        Theme::Vibe => 4,
    };

    lines.push(Line::from(vec![Span::styled("  Theme: ", colors.dim())]));

    // Theme options as horizontal buttons
    let mut theme_spans = vec![Span::raw("    ")];
    for (i, &theme) in themes.iter().enumerate() {
        let style = if i == current_theme_idx {
            colors.selected().add_modifier(Modifier::BOLD)
        } else if selected_field == 0 && i == current_theme_idx {
            colors.accent()
        } else {
            colors.normal()
        };

        if i == current_theme_idx {
            theme_spans.push(Span::styled(format!("[{}]", theme), style));
        } else {
            theme_spans.push(Span::styled(format!(" {} ", theme), style));
        }
        theme_spans.push(Span::raw(" "));
    }
    lines.push(Line::from(theme_spans));

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  Keyboard shortcuts:",
        colors.dim(),
    )]));
    lines.push(Line::from(vec![
        Span::raw("    "),
        Span::styled("t", colors.accent()),
        Span::raw(" - Cycle theme"),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    "),
        Span::styled("Tab", colors.accent()),
        Span::raw(" - Switch view (Issues/Kanban)"),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    "),
        Span::styled("S", colors.accent()),
        Span::raw(" - Sync with remote"),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    "),
        Span::styled("n", colors.accent()),
        Span::raw(" - New issue"),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    "),
        Span::styled("d", colors.accent()),
        Span::raw(" - Delete issue"),
    ]));
    lines.push(Line::from(vec![
        Span::raw("    "),
        Span::styled("Enter", colors.accent()),
        Span::raw(" - Open issue details"),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "  ─── Recommended Fonts ───",
        colors.dim(),
    )]));
    lines.push(Line::from(vec![
        Span::raw("    For best icon support, use a "),
        Span::styled("Nerd Font", colors.accent().add_modifier(Modifier::BOLD)),
        Span::raw(":"),
    ]));
    lines.push(Line::from(vec![Span::styled(
        "    • JetBrainsMono Nerd Font",
        colors.normal(),
    )]));
    lines.push(Line::from(vec![Span::styled(
        "    • FiraCode Nerd Font",
        colors.normal(),
    )]));
    lines.push(Line::from(vec![Span::styled(
        "    • Hack Nerd Font",
        colors.normal(),
    )]));
    lines.push(Line::from(vec![
        Span::raw("    "),
        Span::styled("https://www.nerdfonts.com", colors.dim()),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Icon test: ", colors.dim()),
        Span::styled("       ", colors.accent()), // Nerd Font icons
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Press ", colors.dim()),
        Span::styled("Esc", colors.accent()),
        Span::styled(" or ", colors.dim()),
        Span::styled("O", colors.accent()),
        Span::styled(" to close", colors.dim()),
    ]));

    let content = Paragraph::new(lines).block(
        Block::default()
            .title(Span::styled(
                " ⚙ Settings ",
                colors.accent().add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(colors.accent()),
    );

    frame.render_widget(content, area);

    area
}

/// Create a centered rect
fn centered_rect(r: Rect, percent_x: u16, percent_y: u16) -> Rect {
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
