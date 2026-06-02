use super::AppState;
use crate::prelude::*;
use anylm::{Content, Message};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

/// Renders the terminal interface
pub fn render_tui(f: &mut Frame, app: &mut AppState) {
    let area = f.area();
    let msgs = app.messages.dirty_get();

    // --- 1. ВЫЧИСЛЕНИЕ ДИНАМИЧЕСКИХ ВЫСОТ ---
    let prefix = " ❯ ";
    let inner_width = area.width.saturating_sub(2) as usize;

    let wrap_options = textwrap::Options::new(inner_width)
        .break_words(true)
        .word_separator(textwrap::WordSeparator::AsciiSpace);

    let full_text = str!("{prefix}{}", app.input);
    let wrapped_lines = textwrap::wrap(&full_text, wrap_options.clone());
    let input_line_count = wrapped_lines.len().max(1) as u16;
    let dynamic_input_height = (input_line_count + 2).clamp(3, 8);

    // Вычисляем высоту футера (статус/подсказки) с поддержкой 2 строк
    let footer_height = calculate_footer_height(app, area.width.saturating_sub(2) as usize);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),                    // Хедер под текущий путь (без рамок)
            Constraint::Min(1),                       // Чат занимает всё свободное место
            Constraint::Length(dynamic_input_height), // Инпут сфокусирован снизу
            Constraint::Length(footer_height),        // Динамический статус под инпутом
        ])
        .split(area);

    app.chat_area = chunks[1];
    app.input_area = chunks[2];

    // --- 2. РЕНДЕР КОМПОНЕНТОВ ---
    render_header(f, chunks[0]);
    render_chat(f, chunks[1], app, &msgs);
    render_input(f, chunks[2], app, &wrap_options, prefix);
    render_footer(f, chunks[3], app);

    // Чистим лог-гард
    drop(msgs);
}

/// Рендеринг минималистичного хедера без рамок с указанием текущего пути
fn render_header(f: &mut Frame, area: Rect) {
    let current_path = std::env::current_dir()
        .map(|p| {
            let path_str = p.display().to_string();
            let home_dir = path!("~/");
            let home_str = home_dir.display().to_string();
            if path_str.starts_with(&home_str) {
                return path_str.replacen(&home_str, "~/", 1);
            }
            path_str
        })
        .unwrap_or_else(|_| "Unknown".to_string());

    let header_line = Line::from(vec![
        Span::raw(" "),
        Span::styled(current_path, Style::default().fg(Color::DarkGray).italic()),
    ]);

    f.render_widget(Paragraph::new(header_line), area);
}

/// Рендеринг чистого чата без внешних границ
fn render_chat(f: &mut Frame, area: Rect, app: &mut AppState, msgs: &[Message]) {
    let chat_inner_width = area.width;
    let chat_inner_height = area.height;

    let mut history: Vec<Line> = Vec::new();

    for msg in msgs.iter() {
        let text_content = msg
            .content
            .iter()
            .filter_map(|p| {
                if let Content::Text { text } = p {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect::<String>()
            .trim()
            .to_string();

        if text_content.is_empty() {
            continue;
        }

        // Метаданные пользователя
        if msg.role.is_user() {
            let meta_line = Line::from(vec![Span::styled(
                str!(
                    "{}",
                    msg.timestamp
                        .map(|t| t.with_timezone(&Local).format("%A %I:%M %p").to_string())
                        .unwrap_or_else(|| "??:??:??".to_string())
                ),
                Style::default().bg(Color::Cyan).fg(Color::Black).bold(),
            )]);

            history.push(Line::raw(""));
            history.push(meta_line);
        } else if !history.is_empty() {
            history.push(Line::raw(""));
        }

        let msg_lines = super::parse_markdown(&text_content, chat_inner_width as usize);

        if msg.role.is_user() {
            for line in msg_lines {
                history.push(
                    line.patch_style(
                        Style::default()
                            .bg(Color::Rgb(15, 15, 15))
                            .fg(Color::DarkGray),
                    ),
                );
            }
        } else if msg.role.is_tool() {
            for line in msg_lines {
                history.push(line.patch_style(Style::default().dim()));
            }
        } else {
            history.extend(msg_lines);
        }
    }

    history.push(Line::raw(""));

    // Управление скроллом
    let total_lines = history.len() as u16;
    let max_chat_scroll = total_lines.saturating_sub(chat_inner_height);
    app.chat_scroll = app.chat_scroll.min(max_chat_scroll);

    f.render_widget(Paragraph::new(history).scroll((app.chat_scroll, 0)), area);
}

/// Рендеринг инпут-бокса с прогресс-баром слева от имени модели
fn render_input(
    f: &mut Frame,
    area: Rect,
    app: &mut AppState,
    wrap_options: &textwrap::Options,
    prefix: &str,
) {
    let input_inner_height = area.height.saturating_sub(2);

    // Скроллинг инпута при многострочном вводе
    let text_before_cursor = &app.input[..app.input_cursor];
    let text_temp = str!("{}{}_", prefix, text_before_cursor);
    let wrapped_before = textwrap::wrap(&text_temp, wrap_options.clone());
    let cursor_row = (wrapped_before.len() as u16).saturating_sub(1);

    if cursor_row >= app.input_scroll + input_inner_height {
        app.input_scroll = cursor_row - input_inner_height + 1;
    }
    if cursor_row < app.input_scroll {
        app.input_scroll = cursor_row;
    }

    // --- ПОДГОТОВКА ДИНАМИЧЕСКОГО СТИЛЯ ДЛЯ ВСЕГО БЛОКА ---
    // Если ИИ думает — всё становится белым, иначе — циановым
    let (input_color, title_style) = if app.is_busy {
        (Color::White, Style::default().fg(Color::White).bold())
    } else {
        (Color::Cyan, Style::default().fg(Color::Cyan).bold())
    };
    let input_style = Style::default().fg(input_color);

    // --- ПОДГОТОВКА СЛОЖНЫХ ТИТЛОВ ДЛЯ РАМКИ ---
    let max_tokens = Settings::get()
        .assistant
        .completions
        .max_tokens
        .unwrap_or(1) as usize;
    let tokens_count = super::count_tokens(&app.messages.dirty_get());

    // Левый титл: Название модели и счетчик токенов в цвет текущего состояния
    let title_left = Line::from(vec![
        Span::raw(" "),
        Span::styled(
            Settings::get().assistant.completions.model.clone(),
            title_style,
        ),
        Span::raw(" "),
        Span::styled(
            str!("({}/{})", tokens_count, max_tokens),
            Style::default().fg(if app.is_busy {
                Color::White
            } else {
                Color::Cyan
            }),
        ),
        Span::raw(" "),
    ]);

    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title_left);

    // Титл снизу (показывает статус занятости белым цветом во время генерации)
    if app.is_busy {
        let dots_count = (app.tick_count / 25) % 4;
        let dots = ".".repeat(dots_count as usize);
        let thinking_text = str!(" thinking{dots:<3} ");

        block = block.title_bottom(Span::styled(
            thinking_text,
            Style::default().fg(Color::White),
        ));
    }

    // Рендерим текст внутри инпута (префикс и сам текст будут использовать input_style)
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(prefix, input_style),
            Span::styled(app.input.as_str(), Color::White),
        ]))
        .wrap(Wrap { trim: false })
        .scroll((app.input_scroll, 0))
        .block(block.border_style(input_style)),
        area,
    );

    // --- УСТАНОВКА КУРСОРА ---
    let last_line_len = wrapped_before
        .last()
        .map(|s| s.chars().count())
        .unwrap_or(0);
    let last_line = last_line_len.saturating_sub(1);

    f.set_cursor_position((
        area.x + 1 + last_line as u16,
        area.y + 1 + cursor_row - app.input_scroll,
    ));
}

/// Вычисление высоты футера (1 или 2 строки в зависимости от объема текста)
fn calculate_footer_height(app: &AppState, max_width: usize) -> u16 {
    if let Some(status) = app.status.as_ref().filter(|s| !s.is_empty()) {
        let lines = textwrap::wrap(status, max_width);
        (lines.len().max(1) as u16).clamp(1, 2)
    } else if !app.commands.is_empty() {
        let index = (app.tick_count / 360) as usize % app.commands.len();
        let (cmd, desc) = app.commands[index];
        let full_cmd_text = str!("{} {}", cmd, desc);
        let lines = textwrap::wrap(&full_cmd_text, max_width);
        (lines.len().max(1) as u16).clamp(1, 2)
    } else {
        1
    }
}

/// Рендеринг футера (статус или подсказки)
fn render_footer(f: &mut Frame, area: Rect, app: &AppState) {
    let help_line = if let Some(status) = app.status.as_ref().filter(|s| !s.is_empty()) {
        let parsed_lines = super::parse_markdown(status, area.width as usize);
        let mut spans = vec!["  ".into()];
        for line in parsed_lines {
            for span in line.spans {
                spans.push(span.italic().gray());
            }
        }
        Text::from(Line::from(spans))
    } else if !app.commands.is_empty() {
        let index = (app.tick_count / 360) as usize % app.commands.len();
        let (cmd, desc) = app.commands[index];
        Text::from(Line::from(vec![
            "  ".into(),
            cmd.bold().cyan(),
            " ".into(),
            desc.gray().italic(),
        ]))
    } else {
        Text::from(Line::raw(""))
    };

    f.render_widget(Paragraph::new(help_line).wrap(Wrap { trim: false }), area);
}
