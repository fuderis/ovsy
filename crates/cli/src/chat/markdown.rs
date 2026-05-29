use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

/// The parser state
pub struct ParserState {
    in_code_block: bool,
    in_table: bool,
    table_rows: Vec<Vec<String>>,
}

/// Parses the markdown format
pub fn parse(text: &str, max_width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut state = ParserState {
        in_code_block: false,
        in_table: false,
        table_rows: vec![],
    };

    // foreach text lines:
    for raw_line in text.lines() {
        // code block prefix:
        if let Some(lang_raw) = raw_line.strip_prefix("```") {
            state.in_code_block = !state.in_code_block;

            if state.in_code_block {
                let lang = lang_raw.trim();
                let display_lang = if lang.is_empty() { "code" } else { lang };

                lines.push(Line::from(vec![Span::styled(
                    format!(" {} ", display_lang.to_lowercase()),
                    Style::default()
                        .bg(Color::Rgb(20, 20, 20))
                        .fg(Color::DarkGray)
                        .bold(),
                )]));
            }

            continue;
        }

        // code block contents:
        if state.in_code_block {
            let wrapped = textwrap::wrap(raw_line, max_width);

            if wrapped.is_empty() {
                lines.push(Line::from(vec![Span::styled(
                    " ",
                    Style::default().bg(Color::Rgb(45, 45, 45)),
                )]));
            } else {
                lines.extend(wrapped.into_iter().map(|part| {
                    Line::from(vec![Span::styled(
                        part.into_owned(),
                        Style::default().bg(Color::Rgb(45, 45, 45)).fg(Color::White),
                    )])
                }));
            }

            continue;
        }

        // line break:
        if raw_line.contains("<br>") || raw_line.contains("<br/>") {
            lines.push(Line::from(" "));
            continue;
        }

        // table:
        if raw_line.trim().starts_with('|') {
            state.in_table = true;
            let cells: Vec<String> = raw_line
                .split('|')
                .filter(|s| !s.trim().is_empty() || raw_line.contains("||"))
                .map(|s| s.trim().to_string())
                .collect();

            if !cells
                .iter()
                .all(|c| c.chars().all(|ch| ch == '-' || ch == ':'))
            {
                state.table_rows.push(cells);
            }

            continue;
        } else if state.in_table {
            render_table(&mut lines, &mut state, max_width);
            state.in_table = false;
            state.table_rows.clear();
        }

        // horizontal line:
        if raw_line.starts_with("---") {
            lines.push(Line::from(vec![Span::styled(
                "─".repeat(max_width),
                Style::default().fg(Color::DarkGray),
            )]));
            continue;
        }

        // header, list item, quote:
        let (content, base_style, prefix) = match raw_line {
            // header:
            s if s.starts_with("#") => {
                let dots = s.len() - s.trim_start_matches("#").len();
                let rest = s[dots..].trim_start();
                let style = match dots {
                    1 => Style::default().cyan().bold().underlined(),
                    2 | 3 => Style::default().cyan().bold(),
                    4 => Style::default().white().bold(),
                    5 => Style::default().white().bold().italic(),
                    _ => Style::default().white().italic(),
                };
                (rest, style, None)
            }

            // list item:
            s if s.starts_with("- ") => (
                s.trim_start_matches("- ").trim(),
                Style::default().white(),
                Some(Span::styled(" • ", Style::default().cyan().bold())),
            ),

            // quote:
            s if s.starts_with("> ") => (
                s.trim_start_matches("> ").trim(),
                Style::default()
                    .bg(Color::Rgb(45, 45, 45))
                    .fg(Color::Rgb(200, 200, 200))
                    .italic(),
                None,
            ),

            s => (s, Style::default().white(), None),
        };

        for wrapped_row in textwrap::wrap(content, max_width) {
            let mut spans = Vec::new();
            if let Some(ref p) = prefix {
                spans.push(p.clone());
            }
            spans.extend(parse_inline_styles(&wrapped_row, base_style));
            lines.push(Line::from(spans));
        }
    }

    // render table:
    if state.in_table {
        render_table(&mut lines, &mut state, max_width);
    }

    lines
}

/// Renders the markdown table
fn render_table(lines: &mut Vec<Line>, state: &mut ParserState, max_width: usize) {
    if state.table_rows.is_empty() {
        return;
    }

    let col_count = state.table_rows[0].len();
    let mut ideal_widths = vec![0; col_count];

    for row in &state.table_rows {
        for (i, cell) in row.iter().enumerate().take(col_count) {
            ideal_widths[i] = ideal_widths[i].max(strip_markdown(cell).width());
        }
    }

    let available_width = max_width.saturating_sub(2 + (col_count * 3));
    let total_ideal_width: usize = ideal_widths.iter().sum();
    let mut final_widths = ideal_widths.clone();

    if total_ideal_width > available_width && available_width > 0 {
        for i in 0..col_count {
            final_widths[i] = ((ideal_widths[i] * available_width) / total_ideal_width).max(1);
        }
    }

    let make_border = |left, mid, right, line_char: &str| {
        let mut border = vec![Span::styled(left, Style::default().dim())];
        for (i, &w) in final_widths.iter().enumerate() {
            border.push(Span::styled(
                line_char.repeat(w + 2),
                Style::default().dim(),
            ));
            if i < col_count - 1 {
                border.push(Span::styled(mid, Style::default().dim()));
            }
        }
        border.push(Span::styled(right, Style::default().dim()));
        Line::from(border)
    };

    lines.push(make_border("┌", "┬", "┐", "─"));

    let rows_len = state.table_rows.len();
    for (r_idx, row) in state.table_rows.iter().enumerate() {
        let mut wrapped_cells = Vec::new();
        let mut max_cell_lines = 1;

        for cell in row.iter().take(col_count) {
            let wrapped: Vec<String> =
                textwrap::wrap(&strip_markdown(cell), final_widths[wrapped_cells.len()])
                    .into_iter()
                    .map(|s| s.into_owned())
                    .collect();
            max_cell_lines = max_cell_lines.max(wrapped.iter().len());
            wrapped_cells.push(wrapped);
        }

        for line_idx in 0..max_cell_lines {
            let mut line_spans = vec![Span::styled("│ ", Style::default().dim())];
            for (i, wrapped_lines) in wrapped_cells.iter().enumerate() {
                let content = wrapped_lines.get(line_idx).cloned().unwrap_or_default();
                let base_style = if r_idx == 0 {
                    Style::default().cyan().bold()
                } else {
                    Style::default().white()
                };

                line_spans.extend(parse_inline_styles(&content, base_style));
                line_spans.push(Span::styled(
                    " ".repeat(final_widths[i].saturating_sub(content.width())),
                    base_style,
                ));
                line_spans.push(Span::styled(" │ ", Style::default().dim()));
            }
            lines.push(Line::from(line_spans));
        }

        if r_idx < rows_len - 1 {
            lines.push(make_border(
                "├",
                "┼",
                "┤",
                if r_idx == 0 { "━" } else { "─" },
            ));
        }
    }
    lines.push(make_border("└", "┴", "┘", "─"));
}

/// Handles the inline styles
fn parse_inline_styles(content: &str, base_style: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut current_text = String::new();
    let mut chars = content.chars().peekable();
    let (mut is_bold, mut is_italic, mut is_inline_code) = (false, false, false);

    let mut flush = |text: &mut String, b, i, c| {
        if !text.is_empty() {
            let mut style = base_style;
            if c {
                style = style.fg(Color::Cyan).bg(Color::Rgb(40, 40, 40));
            } else {
                if b {
                    style = style.bold();
                }
                if i {
                    style = style.italic();
                }
            }
            spans.extend(push_text_with_urls(text, style));
            text.clear();
        }
    };

    while let Some(c) = chars.next() {
        match c {
            '`' => {
                flush(&mut current_text, is_bold, is_italic, is_inline_code);
                is_inline_code = !is_inline_code;
            }
            '*' => {
                let is_double = chars.peek() == Some(&'*');
                if is_double {
                    chars.next();
                }
                flush(&mut current_text, is_bold, is_italic, is_inline_code);
                if is_double {
                    is_bold = !is_bold;
                } else {
                    is_italic = !is_italic;
                }
            }
            _ => current_text.push(c),
        }
    }

    flush(&mut current_text, is_bold, is_italic, is_inline_code);
    spans
}

/// Handles the markdown URL links
fn push_text_with_urls(text: &str, base_style: Style) -> Vec<Span<'static>> {
    text.split_inclusive(|c: char| c.is_whitespace())
        .map(|word| {
            let trimmed = word.trim();
            if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
                Span::styled(word.to_string(), base_style.fg(Color::Cyan).underlined())
            } else {
                Span::styled(word.to_string(), base_style)
            }
        })
        .collect()
}

/// Strips markdown spec-symbols
fn strip_markdown(text: &str) -> String {
    text.replace("**", "")
        .replace("__", "")
        .replace("*", "")
        .replace("_", "")
        .replace("`", "")
        .replace("<br>", "")
        .replace("<br/>", "")
}
