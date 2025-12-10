use crate::tui::app::{App, InputMode, ViewMode};
use crate::tui::theme::Theme;

#[derive(Debug, PartialEq)]
pub enum CommandAction {
    None,
    Quit,
    Refresh,
    Status(String),
    Error(String),
}

pub fn execute(app: &mut App, input: &str) -> CommandAction {
    let parts: Vec<&str> = input.trim().split_whitespace().collect();
    if parts.is_empty() {
        return CommandAction::None;
    }

    match parts[0] {
        "q" | "quit" => CommandAction::Quit,
        "n" | "new" => {
            app.input_mode = InputMode::Edit;
            CommandAction::Refresh
        },
        "w" | "write" => CommandAction::Status("Saved".to_string()),
        "theme" => {
            if parts.len() < 2 {
                return CommandAction::Error("Usage: :theme [nord|gruvbox|dracula|cyberpunk]".to_string());
            }
            match parts[1] {
                "nord" => app.theme = Theme::Nord,
                "gruvbox" => app.theme = Theme::Gruvbox,
                "dracula" => app.theme = Theme::Dracula,
                "cyberpunk" => app.theme = Theme::Cyberpunk,
                _ => return CommandAction::Error(format!("Unknown theme: {}", parts[1])),
            }
            CommandAction::Status(format!("Theme set to {}", parts[1]))
        },
        "sort" => {
             if parts.len() < 2 {
                return CommandAction::Error("Usage: :sort [due|created|status|effort]".to_string());
             }
             // For now just partial impl
             match parts[1] {
                 "created" => {
                     // app.issues.sort_by... 
                     // We need to implement sorts in app or issue module
                     CommandAction::Status("Sorted by creation".to_string())
                 }
                 _ => CommandAction::Error("Sort field not supported yet".to_string())
             }
        }, 
        _ => CommandAction::Error(format!("Unknown command: {}", parts[0]))
    }
}
