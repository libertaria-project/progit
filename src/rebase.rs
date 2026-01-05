// SPDX-License-Identifier: EUPL-1.2
// Copyright (c) 2025 Markus Maiwald

//! Interactive Rebase Editor
//!
//! Provides a TUI for editing git-rebase-todo files.
//! Replaces the text editor with a visual, interactive experience.

use anyhow::{Context, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::fs;
use std::io;
// Path unused

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Pick,
    Reword,
    Edit,
    Squash,
    Fixup,
    Drop,
}

impl Action {
    fn as_str(&self) -> &'static str {
        match self {
            Action::Pick => "pick",
            Action::Reword => "reword",
            Action::Edit => "edit",
            Action::Squash => "squash",
            Action::Fixup => "fixup",
            Action::Drop => "drop",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "p" | "pick" => Some(Action::Pick),
            "r" | "reword" => Some(Action::Reword),
            "e" | "edit" => Some(Action::Edit),
            "s" | "squash" => Some(Action::Squash),
            "f" | "fixup" => Some(Action::Fixup),
            "d" | "drop" => Some(Action::Drop),
            _ => None,
        }
    }

    fn next(&self) -> Self {
        match self {
            Action::Pick => Action::Reword,
            Action::Reword => Action::Edit,
            Action::Edit => Action::Squash,
            Action::Squash => Action::Fixup,
            Action::Fixup => Action::Drop,
            Action::Drop => Action::Pick,
        }
    }

    fn prev(&self) -> Self {
        match self {
            Action::Pick => Action::Drop,
            Action::Reword => Action::Pick,
            Action::Edit => Action::Reword,
            Action::Squash => Action::Edit,
            Action::Fixup => Action::Squash,
            Action::Drop => Action::Fixup,
        }
    }

    fn color(&self) -> Color {
        match self {
            Action::Pick => Color::Green,
            Action::Reword => Color::Yellow,
            Action::Edit => Color::Blue,
            Action::Squash => Color::Magenta,
            Action::Fixup => Color::Cyan,
            Action::Drop => Color::Red,
        }
    }
    
    fn icon(&self) -> &'static str {
        match self {
            Action::Pick => "⛏️ ",
            Action::Reword => "📝",
            Action::Edit => "🔨",
            Action::Squash => "🤏",
            Action::Fixup => "🍬",
            Action::Drop => "🗑️ ",
        }
    }
}

pub struct RebaseEntry {
    pub action: Action,
    pub hash: String,
    pub message: String,
}

pub struct RebaseApp {
    pub entries: Vec<RebaseEntry>,
    pub selected: usize,
    pub path: String,
}

impl RebaseApp {
    pub fn new(path: String) -> Self {
        Self {
            entries: Vec::new(),
            selected: 0,
            path,
        }
    }

    pub fn load(&mut self) -> Result<()> {
        let content = fs::read_to_string(&self.path)
            .with_context(|| format!("Failed to read rebase file: {}", self.path))?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse line: action hash message
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                if let Some(action) = Action::from_str(parts[0]) {
                    let hash = parts[1].to_string();
                    let message = parts[2..].join(" ");
                    self.entries.push(RebaseEntry {
                        action,
                        hash,
                        message,
                    });
                }
            }
        }
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let mut content = String::new();
        for entry in &self.entries {
            content.push_str(&format!(
                "{} {} {}\n",
                entry.action.as_str(),
                entry.hash,
                entry.message
            ));
        }
        // Preserve comments? For now, we rewrite cleanly. Git preserves comments usually but this is Todo file.
        // It's better to just write the tasks.
        
        fs::write(&self.path, content)
            .with_context(|| format!("Failed to write rebase file: {}", self.path))?;
        Ok(())
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.entries.swap(self.selected, self.selected - 1);
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected < self.entries.len() - 1 {
            self.entries.swap(self.selected, self.selected + 1);
            self.selected += 1;
        }
    }
}

/// Run the interactive rebase editor
pub fn run(path: &str) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = RebaseApp::new(path.to_string());
    if let Err(e) = app.load() {
        // Cleanup on load error
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
        return Err(e);
    }

    let mut res = Ok(());

    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    // Abort? Or just quit without saving? 
                    // Git interprets empty file as abort usually, or we should ask.
                    // For now, let's treat Q as "Do nothing / Cancel" -> But users might want to abort rebase.
                    // Let's implement 'Ctrl+C' as abort.
                    // Q/Esc exit? No, we need explicit "Done" vs "Abort".
                    // Let's make "Enter" = Save & Quit (Proceed)
                    // "Ctrl+C" = Abort (Exit 1)
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    // Abort
                    res = Err(anyhow::anyhow!("Rebase aborted by user"));
                    break;
                }
                 KeyCode::Char('q') => {
                    // Ask for confirmation to abort? Or just abort? 
                    // Let's assume Abort for now, cleaner.
                    res = Err(anyhow::anyhow!("Rebase aborted by user"));
                    break;
                 }
                KeyCode::Enter => {
                    // Save and Exit
                    if let Err(e) = app.save() {
                        res = Err(e);
                    }
                    break;
                }
                
                // Navigation
                KeyCode::Char('j') | KeyCode::Down => {
                    if !app.entries.is_empty() {
                        app.selected = (app.selected + 1).min(app.entries.len() - 1);
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    app.selected = app.selected.saturating_sub(1);
                }
                
                // Move lines
                KeyCode::Char('J') => app.move_down(), // Shift+J
                KeyCode::Char('K') => app.move_up(),   // Shift+K
                
                // Cycle Actions
                KeyCode::Char(' ') | KeyCode::Right | KeyCode::Char('l') => {
                    if let Some(entry) = app.entries.get_mut(app.selected) {
                        entry.action = entry.action.next();
                    }
                }
                KeyCode::Char('h') | KeyCode::Left => {
                   if let Some(entry) = app.entries.get_mut(app.selected) {
                        entry.action = entry.action.prev();
                    } 
                }
                
                // Quick Actions
                KeyCode::Char('p') => app.entries[app.selected].action = Action::Pick,
                KeyCode::Char('r') => app.entries[app.selected].action = Action::Reword,
                KeyCode::Char('e') => app.entries[app.selected].action = Action::Edit,
                KeyCode::Char('s') => app.entries[app.selected].action = Action::Squash,
                KeyCode::Char('f') => app.entries[app.selected].action = Action::Fixup,
                KeyCode::Char('d') => app.entries[app.selected].action = Action::Drop,
                
                _ => {}
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

fn ui(f: &mut ratatui::Frame, app: &RebaseApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Title
    let title = Paragraph::new(Line::from(vec![
        Span::styled(" 🐑 Interactive Rebase ", Style::default().add_modifier(Modifier::BOLD).fg(Color::Magenta)),
        Span::raw(format!("({})", app.entries.len())),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Magenta)));
    f.render_widget(title, chunks[0]);

    // List
    let items: Vec<ListItem> = app.entries.iter().enumerate().map(|(i, entry)| {
        let style = if i == app.selected {
            Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let action_span = Span::styled(
            format!("{} {:<6}", entry.action.icon(), entry.action.as_str()),
            Style::default().fg(entry.action.color())
        );
        
        let hash_span = Span::styled(
            format!(" {} ", &entry.hash[..7.min(entry.hash.len())]),
            Style::default().fg(Color::Yellow)
        );
        
        let msg_span = Span::raw(&entry.message);

        let line = Line::from(vec![
            action_span,
            hash_span,
            msg_span
        ]);

        ListItem::new(line).style(style)
    }).collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Commits "));
    
    // Auto-scroll logic: centered around selected
    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(app.selected));
    
    f.render_stateful_widget(list, chunks[1], &mut state);

    // Help
    let help_text = "j/k:nav │ J/K:move │ Space:cycle action │ p/r/e/s/f/d:quick action │ Enter:SAVE │ Ctrl+C:ABORT";
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL).title(" Help "));
    f.render_widget(help, chunks[2]);
}
