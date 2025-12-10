# TUI Feature

Terminal user interface with Ratatui + Crossterm.

## Structure

| File | Purpose |
|------|---------|
| `app.rs` | App state machine, mode management |
| `input.rs` | Vim-style keyboard handling (hjkl) |
| `theme.rs` | Color schemes (Nord, Gruvbox) |
| `widget_issues.rs` | Issue table widget |
| `widget_kanban.rs` | Kanban board view |
| `widget_status.rs` | Status bar + sprint timer |

## Keybindings

| Key | Action |
|-----|--------|
| `j/↓` | Move down |
| `k/↑` | Move up |
| `h/←` | Move left (kanban) |
| `l/→` | Move right (kanban) |
| `Enter` | Edit selected issue |
| `Space` | Cycle status |
| `n` | New issue |
| `d` | Delete issue |
| `Tab` | Toggle list/kanban view |
| `q` | Quit |
| `/` | Search |

## Views

1. **List View** - Sortable table of all issues
2. **Kanban View** - 3-column board (Backlog | In Progress | Done)
