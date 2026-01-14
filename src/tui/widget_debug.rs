use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders},
    Frame,
};
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerWidget};

use crate::tui::app::App;
// ThemeColors unused here

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let colors = app.theme.colors();
    let engine = &app.theme_engine;

    let debug_widget = TuiLoggerWidget::default()
        .block(
            Block::default()
                .title(" 🐞 Debug Console ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(engine.get("debug.border", colors.error())),
        )
        .style_error(Style::default().fg(Color::Red))
        .style_warn(Style::default().fg(Color::Yellow))
        .style_info(Style::default().fg(Color::Cyan))
        .style_debug(Style::default().fg(Color::Green))
        .style_trace(Style::default().fg(Color::Magenta))
        .output_separator(':')
        .output_timestamp(Some("%H:%M:%S".to_string()))
        .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
        .output_target(false)
        .output_file(false)
        .output_line(false);

    frame.render_widget(debug_widget, area);
}
