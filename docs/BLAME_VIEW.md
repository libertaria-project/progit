# Git Blame View

## Overview

The Blame View provides a powerful terminal-based interface to understand code authorship and change history using `git blame`. It offers two specialized perspectives optimized for different roles.

## Accessing Blame View

1. **Via Fuzzy Palette**: Press `Ctrl+P`, type a filename, then press `b` to blame that file
2. **Navigation**: Use `j/k` or arrow keys to scroll through lines
3. **Mode Toggle**: Press `m` to switch between Manager and Lead Dev modes
4. **Exit**: Press `q` to return to previous view

## Viewing Modes

### Manager Mode (Default)
**Focus**: Who made changes and when

| Column | Description |
|--------|-------------|
| **Author** | Developer who made the change |
| **Date** | When the change was committed |
| **Content** | The actual line of code |

**Use Case**: Understanding team contributions, tracking recent changes, identifying stale code.

### Lead Dev Mode
**Focus**: Technical details and commit context

| Column | Description |
|--------|-------------|
| **Author** | Developer who made the change |
| **Commit** | Short commit hash (first 8 chars) |
| **Content** | The actual line of code |

**Use Case**: Investigating specific changes, preparing for code reviews, debugging issues.

## Features

- **Live Git Integration**: Runs `git blame --porcelain` on demand
- **Color Coding**: 
  - Accent color for author names
  - Dimmed styling for metadata
  - Normal color for code content
- **Keyboard Navigation**: Full vim-style navigation (`j/k`, scroll)
- **Theme Support**: Respects your current ProGit theme
- **Efficient Parsing**: Structured parsing of porcelain output for reliability

## Backend Architecture

### Data Structures

```rust
pub struct BlameLine {
    pub line_number: usize,
    pub commit_hash: String,
    pub original_line: usize,
    pub author: String,
    pub author_mail: String,
    pub author_time: DateTime<Utc>,
    pub summary: String,
    pub content: String,
}

pub struct BlameInfo {
    pub file_path: String,
    pub lines: Vec<BlameLine>,
}
```

### Parser

The blame parser (`src/git/blame.rs`) handles `git blame --porcelain` output:

- **Commit Metadata Caching**: Avoids duplicate parsing for repeated commits
- **Line-by-line Processing**: Handles large files efficiently
- **Error Resilient**: Returns `Result<BlameInfo>` with proper error context

## Use Cases

### 1. Code Review Preparation
Before reviewing a PR, blame the changed files to understand:
- Who originally wrote the code being modified
- How old the existing code is
- What the original intent was (from commit messages)

### 2. Bug Investigation
When investigating a bug:
1. Open the file in Blame View
2. Navigate to the problematic line
3. See who introduced the code and when
4. Use the commit hash to review the full change

### 3. Team Velocity Tracking
Managers can use Blame View to:
- See contribution patterns across files
- Identify code ownership
- Spot files that haven't been touched in a long time

### 4. Onboarding
New team members can use Blame View to:
- Understand who to ask about specific code sections
- See the history of changes in critical files
- Learn coding patterns from experienced developers

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `m` | Toggle Manager/Lead Dev mode |
| `q` | Exit to previous view |

## Implementation Details

### File Location
- Backend: `src/git/blame.rs`
- TUI Widget: `src/tui/widget_blame.rs`
- Integration: `src/tui/app.rs` (load_blame method)

### Performance Considerations
- Blame data is loaded on-demand (not cached)
- Suitable for files up to several thousand lines
- For very large files, consider using Git directly

### Future Enhancements
- [ ] Commit message preview on hover/selection
- [ ] Jump to commit in git log
- [ ] Filter by author
- [ ] Highlight recent changes (last 7/30 days)
- [ ] Copy commit hash to clipboard
- [ ] Integrated diff view from blame line

## Troubleshooting

**Q: Blame view shows "Failed to load blame"**  
A: Ensure you're running ProGit from within a Git repository and the file exists in the current branch.

**Q: Date formatting looks wrong**  
A: Blame View uses UTC timestamps from Git. Local time zone conversion coming soon.

**Q: Can I blame files not tracked by Git?**  
A: No, only files tracked by Git can be blamed.

## Related Documentation

- [Branching Guide](BRANCHING.md) - Understanding branch workflows
- [Styling Guide](styling.md) - Theme customization
- [SDK](SDK.md) - Extending ProGit with plugins
