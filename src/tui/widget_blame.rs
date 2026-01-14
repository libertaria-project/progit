use crate::tui::app::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};
// ThemeColors unused here
use crate::git::blame::BlameInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlameMode {
    Manager, // Author / Date / Content
    LeadDev, // Author / Hash / Content
}

#[derive(Debug, Clone)]
pub struct BlameState {
    pub info: Option<BlameInfo>,
    pub mode: BlameMode,
    pub scroll: usize, // Table state scroll is handled by TableState, but we might need offset
    pub table_state: TableState,
    pub loading: bool,
    pub error: Option<String>,
}

impl Default for BlameState {
    fn default() -> Self {
        Self {
            info: None,
            mode: BlameMode::Manager,
            scroll: 0,
            table_state: TableState::default(),
            loading: false,
            error: None,
        }
    }
}

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let colors = app.theme.colors();
    let state = if let Some(s) = &mut app.blame_state {
        s
    } else {
        return; // Should not happen if ViewMode::Blame is active
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(" Git Blame ", colors.accent()))
        .title_alignment(ratatui::layout::Alignment::Center)
        .border_style(colors.normal());

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if state.loading {
        let loading_text = Paragraph::new("Loading blame data...")
            .style(colors.dim())
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(loading_text, inner_area);
        return;
    }

    if let Some(err) = &state.error {
        let error_text = Paragraph::new(format!("Error: {}", err))
            .style(colors.error())
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(error_text, inner_area);
        return;
    }

    let info = if let Some(info) = &state.info {
        info
    } else {
        let no_data = Paragraph::new("No blame information available.")
            .style(colors.dim())
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(no_data, inner_area);
        return;
    };

    // Header based on mode
    let headers = match state.mode {
        BlameMode::Manager => Row::new(vec![
            Cell::from("Line").style(colors.dim()),
            Cell::from("Author").style(colors.accent()),
            Cell::from("Date").style(colors.accent()),
            Cell::from("Content").style(colors.normal()),
        ]),
        BlameMode::LeadDev => Row::new(vec![
            Cell::from("Line").style(colors.dim()),
            Cell::from("Hash").style(colors.accent()),
            Cell::from("Author").style(colors.normal()),
            Cell::from("Content").style(colors.normal()),
        ]),
    };

    // Rows
    let rows: Vec<Row> = info
        .lines
        .iter()
        .map(|line| {
            let base_style = match state.mode {
                BlameMode::Manager => {
                    // Highlight recent changes? For now just normal.
                    colors.normal()
                }
                BlameMode::LeadDev => colors.normal(),
            };

            match state.mode {
                BlameMode::Manager => {
                    let date_str = line.author_time.format("%Y-%m-%d").to_string();
                    Row::new(vec![
                        Cell::from(line.line_number.to_string()).style(colors.dim()),
                        Cell::from(line.author.clone()),
                        Cell::from(date_str),
                        Cell::from(line.content.clone()),
                    ])
                }
                BlameMode::LeadDev => {
                    let hash_short = if line.commit_hash.len() > 8 {
                        &line.commit_hash[0..8]
                    } else {
                        &line.commit_hash
                    };
                    Row::new(vec![
                        Cell::from(line.line_number.to_string()).style(colors.dim()),
                        Cell::from(hash_short.to_string()).style(colors.warning()),
                        Cell::from(line.author.clone()),
                        Cell::from(line.content.clone()),
                    ])
                }
            }
            .style(base_style)
        })
        .collect();

    let constraints = match state.mode {
        BlameMode::Manager => vec![
            Constraint::Length(5),  // Line
            Constraint::Length(20), // Author
            Constraint::Length(12), // Date
            Constraint::Min(10),    // Content
        ],
        BlameMode::LeadDev => vec![
            Constraint::Length(5),  // Line
            Constraint::Length(10), // Hash
            Constraint::Length(20), // Author
            Constraint::Min(10),    // Content
        ],
    };

    let table = Table::new(rows, constraints)
        .header(headers.style(Style::default().add_modifier(Modifier::BOLD)))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_stateful_widget(table, inner_area, &mut state.table_state);
}
