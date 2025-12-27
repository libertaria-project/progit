use anyhow::{Result, Context, anyhow};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders},
};
use git2::{Repository, DiffOptions};
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style as SyntectStyle};
use syntect::easy::HighlightLines;
use std::path::{Path};
use once_cell::sync::Lazy;

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(|| SyntaxSet::load_defaults_newlines());
static THEME_SET: Lazy<ThemeSet> = Lazy::new(|| ThemeSet::load_defaults());

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
pub struct FileDiff {
    pub path: String,
    pub is_binary: bool,
    pub lines: Vec<AlignedLine>,
    pub additions: usize,
    pub deletions: usize,
    pub collapsed: bool, // Added field
}

#[derive(Debug, Clone)]
pub struct DiffState {
    pub files: Vec<FileDiff>,
    pub selected_file: usize,
    pub scroll: u16,
    pub reference: String,
}

impl DiffState {
    pub fn new(reference: String) -> Self {
        Self {
            files: Vec::new(),
            selected_file: 0,
            scroll: 0,
            reference,
        }
    }

    pub fn load(&mut self, repo_root: &Path) -> Result<()> {
        let repo = Repository::open(repo_root).context("Failed to open repository")?;
        let mut opts = DiffOptions::new();
        opts.context_lines(3);
        opts.interhunk_lines(1);

        let diff = if self.reference.is_empty() {
            repo.diff_index_to_workdir(None, Some(&mut opts))?
        } else {
            let obj = repo.revparse_single(&self.reference)?;
            let tree = obj.as_tree().ok_or_else(|| anyhow!("Reference is not a tree"))?;
            repo.diff_tree_to_workdir_with_index(Some(&tree), Some(&mut opts))?
        };

        self.parse_git2_diff(&diff)?;
        self.align_lines();
        Ok(())
    }

    fn parse_git2_diff(&mut self, diff: &git2::Diff) -> Result<()> {
        self.files.clear();
        
        // Use a temporary vector to avoid borrow checker issues with closures
        let mut file_diffs: Vec<FileDiff> = Vec::new();
        
        // We'll iterate manually or use print/print_callback if foreach is too troublesome
        // But Diff::foreach is designed for this. We just need to handle state correctly.
        // We can use RefCell for the files vector to allow interior mutability
        
        use std::cell::RefCell;
        let files_cell = RefCell::new(&mut file_diffs);
        
        diff.foreach(
            &mut |delta, _| {
                let path = delta.new_file().path().and_then(|p| p.to_str()).unwrap_or("unknown").to_string();
                files_cell.borrow_mut().push(FileDiff {
                    path,
                    is_binary: delta.flags().contains(git2::DiffFlags::BINARY),
                    lines: Vec::new(),
                    additions: 0,
                    deletions: 0,
                    collapsed: false,
                });
                true
            },
            None,
            None,
            Some(&mut |_, _, line| {
                let mut files = files_cell.borrow_mut();
                if let Some(file) = files.last_mut() {
                    let content = String::from_utf8_lossy(line.content()).to_string();
                    let (line_type, left, right) = match line.origin() {
                        '+' => {
                            file.additions += 1;
                            (DiffLineType::Add, None, line.new_lineno())
                        },
                        '-' => {
                            file.deletions += 1;
                            (DiffLineType::Delete, line.old_lineno(), None)
                        },
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
                            file.lines.push(AlignedLine { left: None, right: Some(diff_line) });
                        },
                        DiffLineType::Delete => {
                            file.lines.push(AlignedLine { left: Some(diff_line), right: None });
                        },
                        _ => {
                            file.lines.push(AlignedLine { left: Some(diff_line.clone()), right: Some(diff_line) });
                        }
                    }
                }
                true
            }),
        )?;

        self.files = file_diffs;
        Ok(())
    }

    pub fn align_lines(&mut self) {
        for file in &mut self.files {
            let mut aligned = Vec::new();
            let mut left_buffer = Vec::new();
            let mut right_buffer = Vec::new();

            let lines = std::mem::take(&mut file.lines);
            for al in lines {
                match (al.left, al.right) {
                    (Some(l), None) => left_buffer.push(l),
                    (None, Some(r)) => right_buffer.push(r),
                    (l, r) => {
                        Self::flush_buffers_static(&mut aligned, &mut left_buffer, &mut right_buffer);
                        aligned.push(AlignedLine { left: l, right: r });
                    }
                }
            }
            Self::flush_buffers_static(&mut aligned, &mut left_buffer, &mut right_buffer);
            file.lines = aligned;
        }
    }

    // Static helper to avoid self-borrowing issues
    fn flush_buffers_static(aligned: &mut Vec<AlignedLine>, left: &mut Vec<DiffLine>, right: &mut Vec<DiffLine>) {
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
    use ratatui::widgets::{List, ListItem, Paragraph};
    use ratatui::layout::{Layout, Constraint, Direction};

    if state.files.is_empty() {
        f.render_widget(
            Paragraph::new("No changes detected.")
                .block(Block::default().borders(Borders::ALL).title(" Diff ")),
            area
        );
        return None;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(area);

    let files: Vec<ListItem> = state.files.iter().enumerate().map(|(i, file)| {
        let style = if i == state.selected_file {
            Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let icon = if file.collapsed { "▶" } else { "▼" };
        ListItem::new(format!("{} {} (+{}/-{})", icon, file.path, file.additions, file.deletions)).style(style)
    }).collect();

    let file_list = List::new(files)
        .block(Block::default().borders(Borders::ALL).title(" Files "));
    f.render_widget(file_list, chunks[0]);

    if let Some(file) = state.files.get(state.selected_file) {
        if file.collapsed {
            f.render_widget(
                Paragraph::new("File collapsed.")
                    .block(Block::default().borders(Borders::ALL).title(" Content ")),
                chunks[1]
            );
            return Some(chunks[0]); // Return file list area
        }

        let diff_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        let syntax = SYNTAX_SET.find_syntax_for_file(&file.path).unwrap_or(None).unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());
        let theme = &THEME_SET.themes["base16-ocean.dark"];

        let mut left_lines = Vec::new();
        let mut right_lines = Vec::new();

        let mut highlighter = HighlightLines::new(syntax, theme);

        for line in &file.lines {
            left_lines.push(render_diff_line(&line.left, &mut highlighter));
            right_lines.push(render_diff_line(&line.right, &mut highlighter));
        }

        let left_para = Paragraph::new(left_lines)
            .block(Block::default().borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM).title(" OLD "))
            .scroll((state.scroll, 0));
        let right_para = Paragraph::new(right_lines)
            .block(Block::default().borders(Borders::ALL).title(" NEW "))
            .scroll((state.scroll, 0));

        f.render_widget(left_para, diff_chunks[0]);
        f.render_widget(right_para, diff_chunks[1]);
    }
    Some(chunks[0]) // Return file list area
}

fn render_diff_line(line: &Option<DiffLine>, highlighter: &mut HighlightLines) -> Line<'static> {
    match line {
        Some(l) => {
            let style = match l.line_type {
                DiffLineType::Add => Style::default().bg(Color::Rgb(0, 50, 0)),
                DiffLineType::Delete => Style::default().bg(Color::Rgb(50, 0, 0)),
                DiffLineType::HunkHeader => Style::default().fg(Color::Cyan),
                _ => Style::default(),
            };

            let line_num = match l.line_number {
                Some(n) => format!("{:4} ", n),
                None => "     ".to_string(),
            };

            let spans = if l.line_type == DiffLineType::HunkHeader || l.line_type == DiffLineType::Header {
                vec![Span::styled(l.content.clone(), style)]
            } else {
                let ranges: Vec<(SyntectStyle, &str)> = highlighter.highlight_line(&l.content, &SYNTAX_SET).unwrap_or_default();
                let mut s = vec![Span::styled(line_num, Style::default().fg(Color::DarkGray))];
                for (style_syn, text) in ranges {
                    let fg = Color::Rgb(style_syn.foreground.r, style_syn.foreground.g, style_syn.foreground.b);
                    s.push(Span::styled(text.to_string(), style.fg(fg)));
                }
                s
            };

            Line::from(spans)
        },
        None => Line::from(Span::styled(" ".repeat(100), Style::default().bg(Color::Rgb(20, 20, 20)))),
    }
}
