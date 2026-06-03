use crate::mr::MRState;
use crate::tui::app::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::Modifier,
    // Line/Span unused
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = app.theme.colors();
    let engine = &app.theme_engine;

    // Empty-state teaching: if there are zero MRs, show help instead of a blank table
    if app.mr_list.is_empty() {
        crate::tui::widget_empty_state::render_mr_list_empty(frame, area, &colors);
        return;
    }

    // Header
    let header_style = engine.get("list.header", colors.header().add_modifier(Modifier::BOLD));
    let header_cells = ["ID", "State", "CI", "Title", "Author", "Branches"]
        .iter()
        .map(|h| Cell::from(*h).style(header_style));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    // Rows
    let rows = app.mr_list.iter().enumerate().map(|(i, mr)| {
        let is_selected = i == app.mr_selected;

        // Styles
        let base_style = if is_selected {
            engine.get("list.selected", colors.selected())
        } else {
            engine.get("list.normal", colors.normal())
        };

        // ID Column
        let id_style = if is_selected {
            base_style.add_modifier(Modifier::BOLD)
        } else {
            engine.get("list.id", colors.dim())
        };
        let id_cell = Cell::from(mr.display_id()).style(id_style);

        // State Column
        let state_icon = if mr.is_draft {
            "📝 Draft"
        } else {
            match mr.state {
                MRState::Open => "🟢 Open",
                MRState::Merged => "🟣 Merged",
                MRState::Closed => "🔴 Closed",
                MRState::Draft => "📝 Draft",
            }
        };
        let state_cell = Cell::from(state_icon).style(base_style);

        // CI Status Column
        let ci_status = match mr.pipeline_status.as_deref() {
            Some("passed") => "✓",
            Some("failed") => "✗",
            Some("running") => "●",
            Some("pending") => "○",
            Some("canceled") => "⊘",
            Some("skipped") => "⊗",
            _ => "–",
        };
        let ci_style = match mr.pipeline_status.as_deref() {
            Some("passed") => engine.get("ci.passed", colors.success()),
            Some("failed") => engine.get("ci.failed", colors.error()),
            Some("running") => engine.get("ci.running", colors.warning()),
            _ => engine.get("ci.unknown", colors.dim()),
        };
        let ci_cell = Cell::from(ci_status).style(ci_style);

        // Title Column
        let title_cell = Cell::from(mr.title.clone()).style(if is_selected {
            base_style.add_modifier(Modifier::BOLD)
        } else {
            base_style
        });

        // Author Column
        let author = mr.author.as_deref().unwrap_or("?");
        let author_cell = Cell::from(author).style(engine.get("list.author", colors.dim()));

        // Branches Column
        let branches = format!("{} ➔ {}", mr.source_branch, mr.target_branch);
        let branches_cell = Cell::from(branches).style(base_style);

        Row::new(vec![
            id_cell,
            state_cell,
            ci_cell,
            title_cell,
            author_cell,
            branches_cell,
        ])
        .style(base_style)
    });

    let widths = [
        Constraint::Length(10),     // ID
        Constraint::Length(12),     // State
        Constraint::Length(4),      // CI
        Constraint::Percentage(35), // Title (reduced from 40)
        Constraint::Length(15),     // Author
        Constraint::Percentage(20), // Branches (reduced from 25)
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::NONE))
        .row_highlight_style(engine.get("list.selected", colors.selected()));

    let mut state = TableState::default();
    state.select(Some(app.mr_selected));

    frame.render_stateful_widget(table, area, &mut state);
}
