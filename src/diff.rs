use anyhow::{anyhow, Context, Result};
use git2::{DiffOptions, Repository};
use once_cell::sync::Lazy;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders},
};
use std::cell::RefCell;
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SyntectStyle, ThemeSet};
use syntect::parsing::SyntaxSet;

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(|| SyntaxSet::load_defaults_newlines());
static THEME_SET: Lazy<ThemeSet> = Lazy::new(|| ThemeSet::load_defaults());

#[derive(Debug, Clone, PartialEq)]
pub enum DiffMode {
    Unstaged,       // Working directory vs index (git diff)
    Staged,         // Index vs HEAD (git diff --cached)
    Custom(String), // Custom comparison (e.g., branch vs HEAD)
}

impl DiffMode {
    pub fn as_str(&self) -> &str {
        match self {
            DiffMode::Unstaged => "Unstaged",
            DiffMode::Staged => "Staged",
            DiffMode::Custom(ref s) => s,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiffLineType {
    Context,
    Add,
    Delete,
    Header,
    HunkHeader,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub content: String,
    pub line_type: DiffLineType,
    pub line_number: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct AlignedLine {
    pub left: Option<DiffLine>,
    pub right: Option<DiffLine>,
}

#[derive(Debug, Clone)]
pub struct DiffLineInfo {
    pub file_path: String,
    pub old_line: Option<usize>,
    pub new_line: Option<usize>,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct Hunk {
    pub header: String,
    pub lines: Vec<AlignedLine>,
    pub collapsed: bool,
}

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub path: String,
    pub is_binary: bool,
    pub hunks: Vec<Hunk>, // Changed from lines to hunks
    pub additions: usize,
    pub deletions: usize,
    pub collapsed: bool,
}

#[derive(Debug, Clone)]
pub struct DiffState {
    pub files: Vec<FileDiff>,
    pub selected_file: usize,
    pub cursor_y: usize, // New: line cursor inside current file
    pub scroll: u16,
    pub mode: DiffMode,
}

impl DiffState {
    pub fn new() -> Self {
        Self::new_with_mode(DiffMode::Unstaged)
    }

    pub fn new_with_mode(mode: DiffMode) -> Self {
        Self {
            files: Vec::new(),
            selected_file: 0,
            cursor_y: 0,
            scroll: 0,
            mode,
        }
    }

    pub fn load(&mut self, repo_root: &Path) -> Result<()> {
        let repo = Repository::open(repo_root).context("Failed to open repository")?;
        let mut opts = DiffOptions::new();
        opts.context_lines(3);
        opts.interhunk_lines(1);

        let diff = match &self.mode {
            DiffMode::Unstaged => {
                // Working directory vs index
                repo.diff_index_to_workdir(None, Some(&mut opts))?
            }
            DiffMode::Staged => {
                // Index vs HEAD (staged changes)
                let head = repo.head()?.peel_to_tree()?;
                let index = repo.index()?;
                repo.diff_tree_to_index(Some(&head), Some(&index), Some(&mut opts))?
            }
            DiffMode::Custom(ref_name) => {
                let obj = repo.revparse_single(ref_name)?;
                let tree = obj
                    .as_tree()
                    .ok_or_else(|| anyhow!("Reference is not a tree"))?;
                repo.diff_tree_to_workdir_with_index(Some(&tree), Some(&mut opts))?
            }
        };

        self.parse_git2_diff(&diff)?;
        self.align_lines();
        Ok(())
    }

    pub fn total_visible_lines(&self) -> usize {
        if let Some(file) = self.files.get(self.selected_file) {
            if file.collapsed {
                return 0;
            }
            file.hunks
                .iter()
                .filter(|h| !h.collapsed)
                .map(|h| h.lines.len())
                .sum()
        } else {
            0
        }
    }

    pub fn clamp_cursor(&mut self) {
        let total = self.total_visible_lines();
        if self.cursor_y >= total {
            self.cursor_y = total.saturating_sub(1);
        }
    }

    pub fn get_selected_line_info(&self) -> Option<DiffLineInfo> {
        let file = self.files.get(self.selected_file)?;
        if file.collapsed {
            return None;
        }

        let mut current_idx = 0;
        for hunk in &file.hunks {
            if hunk.collapsed {
                continue;
            }
            for line in &hunk.lines {
                if current_idx == self.cursor_y {
                    // Prefer right line (new) if available, otherwise left (old)
                    let (old_line, new_line, content) = match (&line.left, &line.right) {
                        (Some(l), Some(r)) => (l.line_number, r.line_number, r.content.clone()),
                        (None, Some(r)) => (None, r.line_number, r.content.clone()),
                        (Some(l), None) => (l.line_number, None, l.content.clone()),
                        (None, None) => (None, None, String::new()),
                    };

                    return Some(DiffLineInfo {
                        file_path: file.path.clone(),
                        old_line,
                        new_line,
                        content,
                    });
                }
                current_idx += 1;
            }
        }
        None
    }

    fn parse_git2_diff(&mut self, diff: &git2::Diff) -> Result<()> {
        self.files.clear();

        // Temporary struct for parsing
        struct TempFile {
            path: String,
            is_binary: bool,
            hunks: Vec<Hunk>,
            additions: usize,
            deletions: usize,
        }

        // Use a temporary vector to avoid borrow checker issues with closures
        let mut temp_files: Vec<TempFile> = Vec::new();
        let files_cell = RefCell::new(&mut temp_files);

        diff.foreach(
            &mut |delta, _| {
                let path = delta
                    .new_file()
                    .path()
                    .and_then(|p| p.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                files_cell.borrow_mut().push(TempFile {
                    path,
                    is_binary: delta.flags().contains(git2::DiffFlags::BINARY),
                    hunks: Vec::new(),
                    additions: 0,
                    deletions: 0,
                });
                true
            },
            None,
            Some(&mut |_, hunk| {
                let mut files = files_cell.borrow_mut();
                if let Some(file) = files.last_mut() {
                    let header = String::from_utf8_lossy(hunk.header()).to_string();
                    file.hunks.push(Hunk {
                        header,
                        lines: Vec::new(),
                        collapsed: false,
                    });
                }
                true
            }),
            Some(&mut |_, _, line| {
                let mut files = files_cell.borrow_mut();
                if let Some(file) = files.last_mut() {
                    if file.hunks.is_empty() {
                        // Fallback: create a dummy hunk if none exists
                        file.hunks.push(Hunk {
                            header: String::from("@@"),
                            lines: Vec::new(),
                            collapsed: false,
                        });
                    }

                    let hunk = file.hunks.last_mut().unwrap();
                    let content = String::from_utf8_lossy(line.content()).to_string();
                    let (line_type, left, right) = match line.origin() {
                        '+' => {
                            file.additions += 1;
                            (DiffLineType::Add, None, line.new_lineno())
                        }
                        '-' => {
                            file.deletions += 1;
                            (DiffLineType::Delete, line.old_lineno(), None)
                        }
                        ' ' => (DiffLineType::Context, line.old_lineno(), line.new_lineno()),
                        'H' => (DiffLineType::HunkHeader, None, None),
                        _ => (DiffLineType::Header, None, None),
                    };

                    let diff_line = DiffLine {
                        content,
                        line_type: line_type.clone(),
                        line_number: left.or(right).map(|n| n as usize),
                    };

                    match line_type {
                        DiffLineType::Add => {
                            hunk.lines.push(AlignedLine {
                                left: None,
                                right: Some(diff_line),
                            });
                        }
                        DiffLineType::Delete => {
                            hunk.lines.push(AlignedLine {
                                left: Some(diff_line),
                                right: None,
                            });
                        }
                        _ => {
                            hunk.lines.push(AlignedLine {
                                left: Some(diff_line.clone()),
                                right: Some(diff_line),
                            });
                        }
                    }
                }
                true
            }),
        )?;

        // Convert temp files to FileDiff
        self.files = temp_files
            .into_iter()
            .map(|temp| FileDiff {
                path: temp.path,
                is_binary: temp.is_binary,
                hunks: temp.hunks,
                additions: temp.additions,
                deletions: temp.deletions,
                collapsed: false,
            })
            .collect();

        Ok(())
    }

    pub fn align_lines(&mut self) {
        for file in &mut self.files {
            for hunk in &mut file.hunks {
                let mut aligned = Vec::new();
                let mut left_buffer = Vec::new();
                let mut right_buffer = Vec::new();

                let lines = std::mem::take(&mut hunk.lines);
                for al in lines {
                    match (al.left, al.right) {
                        (Some(l), None) => left_buffer.push(l),
                        (None, Some(r)) => right_buffer.push(r),
                        (l, r) => {
                            Self::flush_buffers_static(
                                &mut aligned,
                                &mut left_buffer,
                                &mut right_buffer,
                            );
                            aligned.push(AlignedLine { left: l, right: r });
                        }
                    }
                }
                Self::flush_buffers_static(&mut aligned, &mut left_buffer, &mut right_buffer);
                hunk.lines = aligned;
            }
        }
    }

    // Static helper to avoid self-borrowing issues
    fn flush_buffers_static(
        aligned: &mut Vec<AlignedLine>,
        left: &mut Vec<DiffLine>,
        right: &mut Vec<DiffLine>,
    ) {
        let max = std::cmp::max(left.len(), right.len());
        for i in 0..max {
            aligned.push(AlignedLine {
                left: left.get(i).cloned(),
                right: right.get(i).cloned(),
            });
        }
        left.clear();
        right.clear();
    }
}

pub fn render_diff(f: &mut ratatui::Frame, area: Rect, state: &DiffState) -> Option<Rect> {
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::widgets::{List, ListItem, Paragraph};

    if state.files.is_empty() {
        let title = format!(" Diff ({}) - No Changes ", state.mode.as_str());
        f.render_widget(
            Paragraph::new("No changes detected.")
                .block(Block::default().borders(Borders::ALL).title(title)),
            area,
        );
        return None;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(area);

    let files: Vec<ListItem> = state
        .files
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let style = if i == state.selected_file {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let icon = if file.collapsed { "▶" } else { "▼" };
            ListItem::new(format!(
                "{} {} (+{}/-{})",
                icon, file.path, file.additions, file.deletions
            ))
            .style(style)
        })
        .collect();

    let file_title = format!(" Files ({}) ", state.mode.as_str());
    let file_list =
        List::new(files).block(Block::default().borders(Borders::ALL).title(file_title));
    f.render_widget(file_list, chunks[0]);

    if let Some(file) = state.files.get(state.selected_file) {
        if file.collapsed {
            f.render_widget(
                Paragraph::new("File collapsed.")
                    .block(Block::default().borders(Borders::ALL).title(" Content ")),
                chunks[1],
            );
            return Some(chunks[0]); // Return file list area
        }

        let diff_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        let syntax = SYNTAX_SET
            .find_syntax_for_file(&file.path)
            .unwrap_or(None)
            .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());
        let theme = &THEME_SET.themes["base16-ocean.dark"];

        let mut left_lines = Vec::new();
        let mut right_lines = Vec::new();

        let mut highlighter = HighlightLines::new(syntax, theme);

        let mut current_line_idx = 0;
        // Iterate over all hunks and their lines
        for hunk in &file.hunks {
            if !hunk.collapsed {
                for line in &hunk.lines {
                    let is_selected = current_line_idx == state.cursor_y;
                    left_lines.push(render_diff_line(&line.left, &mut highlighter, is_selected));
                    right_lines.push(render_diff_line(&line.right, &mut highlighter, is_selected));
                    current_line_idx += 1;
                }
            }
        }

        let left_para = Paragraph::new(left_lines)
            .block(
                Block::default()
                    .borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM)
                    .title(" OLD "),
            )
            .scroll((state.scroll, 0));
        let right_para = Paragraph::new(right_lines)
            .block(Block::default().borders(Borders::ALL).title(" NEW "))
            .scroll((state.scroll, 0));

        f.render_widget(left_para, diff_chunks[0]);
        f.render_widget(right_para, diff_chunks[1]);
    }
    Some(chunks[0]) // Return file list area
}

fn render_diff_line(
    line: &Option<DiffLine>,
    highlighter: &mut HighlightLines,
    selected: bool,
) -> Line<'static> {
    match line {
        Some(l) => {
            let mut style = match l.line_type {
                DiffLineType::Add => Style::default().bg(Color::Rgb(0, 50, 0)),
                DiffLineType::Delete => Style::default().bg(Color::Rgb(50, 0, 0)),
                DiffLineType::HunkHeader => Style::default().fg(Color::Cyan),
                _ => Style::default(),
            };

            if selected {
                style = style.bg(Color::Rgb(60, 60, 60));
            }

            let line_num = match l.line_number {
                Some(n) => format!("{:4} ", n),
                None => "     ".to_string(),
            };

            let spans =
                if l.line_type == DiffLineType::HunkHeader || l.line_type == DiffLineType::Header {
                    vec![Span::styled(l.content.clone(), style)]
                } else {
                    let ranges: Vec<(SyntectStyle, &str)> = highlighter
                        .highlight_line(&l.content, &SYNTAX_SET)
                        .unwrap_or_default();
                    let mut s = vec![Span::styled(line_num, Style::default().fg(Color::DarkGray))];
                    for (style_syn, text) in ranges {
                        let fg = Color::Rgb(
                            style_syn.foreground.r,
                            style_syn.foreground.g,
                            style_syn.foreground.b,
                        );
                        s.push(Span::styled(text.to_string(), style.fg(fg)));
                    }
                    s
                };

            Line::from(spans)
        }
        None => Line::from(Span::styled(
            " ".repeat(100),
            Style::default().bg(Color::Rgb(20, 20, 20)),
        )),
    }
}
