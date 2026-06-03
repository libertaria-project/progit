// SPDX-License-Identifier: LCL-1.0
// Copyright (c) 2025 Markus Maiwald

//! Markdown rendering for TUI
//!
//! Converts markdown text to styled ratatui Text for terminal display.
//! Supports: headers, bold, italic, code, lists, and blockquotes.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};

/// Render markdown text to styled ratatui Text
pub fn render_markdown(input: &str, base_style: Style) -> Text<'static> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(input, options);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_line: Vec<Span<'static>> = Vec::new();

    // Style stack for nested formatting
    let mut style_stack: Vec<Style> = vec![base_style];
    let mut list_depth: usize = 0;
    let mut in_code_block = false;
    let mut suppress_raw_html_body = false;

    for event in parser {
        match event {
            Event::Start(tag) => {
                let current_style = *style_stack.last().unwrap_or(&base_style);
                match tag {
                    Tag::Heading { level, .. } => {
                        // Flush current line
                        if !current_line.is_empty() {
                            lines.push(Line::from(current_line.clone()));
                            current_line.clear();
                        }
                        // Add header prefix and style
                        let prefix = match level {
                            pulldown_cmark::HeadingLevel::H1 => "# ",
                            pulldown_cmark::HeadingLevel::H2 => "## ",
                            pulldown_cmark::HeadingLevel::H3 => "### ",
                            _ => "#### ",
                        };
                        let header_style =
                            current_style.add_modifier(Modifier::BOLD).fg(Color::Cyan);
                        current_line.push(Span::styled(prefix.to_string(), header_style));
                        style_stack.push(header_style);
                    }
                    Tag::Paragraph => {
                        // Start new paragraph
                        if !lines.is_empty() && !current_line.is_empty() {
                            lines.push(Line::from(current_line.clone()));
                            current_line.clear();
                        }
                    }
                    Tag::Emphasis => {
                        let italic_style = current_style.add_modifier(Modifier::ITALIC);
                        style_stack.push(italic_style);
                    }
                    Tag::Strong => {
                        let bold_style = current_style.add_modifier(Modifier::BOLD);
                        style_stack.push(bold_style);
                    }
                    Tag::Strikethrough => {
                        let strike_style = current_style.add_modifier(Modifier::CROSSED_OUT);
                        style_stack.push(strike_style);
                    }
                    Tag::CodeBlock(_) => {
                        in_code_block = true;
                        if !current_line.is_empty() {
                            lines.push(Line::from(current_line.clone()));
                            current_line.clear();
                        }
                        let code_style = Style::default().fg(Color::Yellow).bg(Color::DarkGray);
                        style_stack.push(code_style);
                    }
                    Tag::List(_) => {
                        list_depth += 1;
                    }
                    Tag::Item => {
                        if !current_line.is_empty() {
                            lines.push(Line::from(current_line.clone()));
                            current_line.clear();
                        }
                        // Add indent and bullet
                        let indent = "  ".repeat(list_depth.saturating_sub(1));
                        current_line.push(Span::styled(
                            format!("{}• ", indent),
                            current_style.fg(Color::Green),
                        ));
                    }
                    Tag::BlockQuote(_) => {
                        if !current_line.is_empty() {
                            lines.push(Line::from(current_line.clone()));
                            current_line.clear();
                        }
                        current_line.push(Span::styled(
                            "│ ".to_string(),
                            Style::default().fg(Color::DarkGray),
                        ));
                        let quote_style =
                            current_style.add_modifier(Modifier::ITALIC).fg(Color::Gray);
                        style_stack.push(quote_style);
                    }
                    Tag::Link { dest_url, .. } => {
                        let link_style = current_style
                            .fg(Color::Blue)
                            .add_modifier(Modifier::UNDERLINED);
                        style_stack.push(link_style);
                        // Store URL for later (simplified: just style the text)
                        let _ = dest_url; // URL not displayed in simple mode
                    }
                    _ => {}
                }
            }
            Event::End(tag_end) => match tag_end {
                TagEnd::Heading(_) => {
                    style_stack.pop();
                    lines.push(Line::from(current_line.clone()));
                    current_line.clear();
                }
                TagEnd::Paragraph => {
                    if !current_line.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                }
                TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link => {
                    style_stack.pop();
                }
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    style_stack.pop();
                    if !current_line.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                }
                TagEnd::List(_) => {
                    list_depth = list_depth.saturating_sub(1);
                }
                TagEnd::Item => {
                    if !current_line.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                }
                TagEnd::BlockQuote(_) => {
                    style_stack.pop();
                    if !current_line.is_empty() {
                        lines.push(Line::from(current_line.clone()));
                        current_line.clear();
                    }
                }
                _ => {}
            },
            Event::Text(text) => {
                if suppress_raw_html_body {
                    continue;
                }
                let current_style = *style_stack.last().unwrap_or(&base_style);
                if in_code_block {
                    // Code blocks: render line by line
                    for (i, line) in text.lines().enumerate() {
                        if i > 0 {
                            lines.push(Line::from(current_line.clone()));
                            current_line.clear();
                            current_line.push(Span::styled("  ".to_string(), current_style));
                        }
                        current_line.push(Span::styled(terminal_safe_text(line), current_style));
                    }
                } else {
                    current_line.push(Span::styled(terminal_safe_text(&text), current_style));
                }
            }
            Event::Code(code) => {
                if suppress_raw_html_body {
                    continue;
                }
                // Inline code
                let code_style = Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD);
                current_line.push(Span::styled(
                    format!("`{}`", terminal_safe_text(&code)),
                    code_style,
                ));
            }
            Event::Html(html) | Event::InlineHtml(html) => {
                update_raw_html_suppression(&html, &mut suppress_raw_html_body);
            }
            Event::SoftBreak => {
                current_line.push(Span::raw(" "));
            }
            Event::HardBreak => {
                lines.push(Line::from(current_line.clone()));
                current_line.clear();
            }
            Event::Rule => {
                if !current_line.is_empty() {
                    lines.push(Line::from(current_line.clone()));
                    current_line.clear();
                }
                lines.push(Line::from(Span::styled(
                    "─".repeat(40),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            _ => {}
        }
    }

    // Flush remaining content
    if !current_line.is_empty() {
        lines.push(Line::from(current_line));
    }

    Text::from(lines)
}

fn terminal_safe_text(input: &str) -> String {
    input.chars().filter(|ch| !ch.is_control()).collect()
}

fn update_raw_html_suppression(html: &str, suppress: &mut bool) {
    let lower = html.to_ascii_lowercase();
    if lower.contains("<script") || lower.contains("<style") {
        *suppress = true;
    }
    if lower.contains("</script") || lower.contains("</style") {
        *suppress = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_plain_text() {
        let text = render_markdown("Hello world", Style::default());
        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_render_header() {
        let text = render_markdown("# Header", Style::default());
        assert!(!text.lines.is_empty());
        // Should contain styled header
    }

    #[test]
    fn test_render_list() {
        let text = render_markdown("- Item 1\n- Item 2", Style::default());
        assert!(text.lines.len() >= 2);
    }

    #[test]
    fn test_render_code() {
        let text = render_markdown("Use `code` here", Style::default());
        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_render_strips_terminal_control_sequences() {
        let text = render_markdown("hello \u{1b}]8;;https://evil\u{7}link", Style::default());
        let rendered = flatten(&text);

        assert!(!rendered.contains('\u{1b}'));
        assert!(!rendered.contains('\u{7}'));
        assert!(rendered.contains("hello "));
        assert!(rendered.contains("link"));
    }

    #[test]
    fn test_render_ignores_raw_html() {
        let text = render_markdown("safe <script>alert(1)</script> text", Style::default());
        let rendered = flatten(&text);

        assert!(rendered.contains("safe"));
        assert!(rendered.contains("text"));
        assert!(!rendered.contains("script"));
        assert!(!rendered.contains("alert"));
    }

    fn flatten(text: &Text<'_>) -> String {
        text.lines
            .iter()
            .flat_map(|line| line.spans.iter().map(|span| span.content.as_ref()))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
