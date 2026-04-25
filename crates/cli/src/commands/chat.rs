use crate::prelude::*;
use anylm::{Content, Message, Role};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ovsy_shared::{Chunk, UserQuery};
use ratatui::{
    Terminal,
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use std::io;
use tokio::sync::mpsc;

/// The user interface app
struct App {
    input: String,
    cursor_position: usize,
    messages: Vec<Message>,
    is_thinking: bool,
    scroll_offset: u16,
    tx: mpsc::UnboundedSender<String>,
}

/// Handles the `chat` command
pub async fn handle() -> Result<()> {
    let (input_tx, mut input_rx) = mpsc::unbounded_channel::<String>();
    let (ui_tx, mut ui_rx) = mpsc::unbounded_channel::<Chunk>();

    // spawn workers:
    let worker_ui_tx = ui_tx.clone();
    tokio::spawn(async move {
        let mut messages: Vec<Message> = Vec::new();
        let port = Settings::get().server.port;
        let client = reqwest::Client::new();

        let max_messages = Settings::get().assistant.max_messages * 2;
        let messages_limit = max_messages * 2;

        while let Some(input) = input_rx.recv().await {
            if input == "/clear" {
                messages.clear();
                let _ = worker_ui_tx.send(Chunk::Answer {
                    answer: "── Context Cleared ──".into(),
                });
                continue;
            }

            messages.push(Message::user(vec![input.into()]));

            let res = client
                .post(format!("http://127.0.0.1:{port}/handle"))
                .json(&UserQuery {
                    user_id: 0,
                    messages: messages.clone(),
                })
                .send()
                .await;

            if let Ok(response) = res {
                let bytes_stream = response.bytes_stream().map(|c| c.map_err(Into::into));
                let mut stream = Stream::read::<Chunk>(bytes_stream);
                let mut full_answer = String::new();

                while let Ok(Some(chunk)) = stream.read().await {
                    if let Chunk::Answer { ref answer } = chunk {
                        full_answer.push_str(answer);
                    }
                    // send chunk to ui:
                    let _ = worker_ui_tx.send(chunk);
                }

                if !full_answer.is_empty() {
                    messages.push(Message::assistant(vec![full_answer.into()]));
                }
            }

            // compressing dialog:
            if messages.len() >= messages_limit {
                let _ = worker_ui_tx.send(Chunk::Think {
                    think: "⚙ Compressing...".into(),
                });

                let compress_count = messages.len() - max_messages;
                let to_compress = messages[..compress_count].to_vec();

                let res = client
                    .post(format!("http://127.0.0.1:{port}/compress"))
                    .json(&UserQuery {
                        user_id: 0,
                        messages: to_compress,
                    })
                    .send()
                    .await;

                if let Ok(res) = res {
                    let bytes_stream = res.bytes_stream().map(|c| c.map_err(Into::into));
                    let mut stream = Stream::read::<Chunk>(bytes_stream);
                    let mut summary = String::new();

                    while let Ok(Some(Chunk::Answer { answer })) = stream.read().await {
                        summary.push_str(&answer);
                    }

                    if !summary.trim().is_empty() {
                        let new_msgs = messages[compress_count..].to_vec();
                        messages = vec![Message::system(vec![
                            format!("Summarized context: {}", summary.trim()).into(),
                        ])];
                        messages.extend(new_msgs);
                    }
                }
            }
        }
    });

    // setup user interface:
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App {
        input: String::new(),
        cursor_position: 0,
        messages: Vec::new(),
        is_thinking: false,
        scroll_offset: 0,
        tx: input_tx,
    };

    // run interface:
    let _ = run_app(&mut terminal, &mut app, &mut ui_rx).await;

    // restore mode & events:
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

/// Runs the UI app
async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    ui_rx: &mut mpsc::UnboundedReceiver<Chunk>,
) -> Result<()>
where
    B::Error: std::fmt::Display,
{
    loop {
        terminal
            .draw(|f| ui(f, app))
            .map_err(|e| str!(e.to_string()))?;

        // read a new chunks:
        while let Ok(chunk) = ui_rx.try_recv() {
            match chunk {
                Chunk::Think { .. } => app.is_thinking = true,
                Chunk::Answer { answer } => {
                    app.is_thinking = false;

                    match app.messages.last_mut() {
                        Some(msg) if msg.role.is_assistant() => {
                            if let Some(Content::Text { text }) = msg.content.get_mut(0) {
                                text.push_str(&answer);
                            }
                        }
                        _ => app.messages.push(Message::assistant(vec![answer.into()])),
                    }

                    app.scroll_offset = app.scroll_offset.saturating_add(1);
                }
                Chunk::Error { error } => {
                    app.is_thinking = false;
                    app.messages
                        .push(Message::system(vec![format!("Error: {error}").into()]));
                }
            }
        }

        // handle events:
        if event::poll(std::time::Duration::from_millis(16))? {
            match event::read()? {
                // terminal resize:
                Event::Resize(_, _) => {
                    terminal.autoresize().ok();
                }

                // keyboard events:
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Esc => return Ok(()),
                    KeyCode::Left => {
                        if app.cursor_position > 0 {
                            app.cursor_position = app
                                .input
                                .char_indices()
                                .filter(|&(i, _)| i < app.cursor_position)
                                .last()
                                .map(|(i, _)| i)
                                .unwrap_or(0);
                        }
                    }
                    KeyCode::Right => {
                        if let Some((i, c)) = app
                            .input
                            .char_indices()
                            .find(|&(i, _)| i == app.cursor_position)
                        {
                            app.cursor_position = i + c.len_utf8();
                        }
                    }
                    KeyCode::Char(c) => {
                        app.input.insert(app.cursor_position, c);
                        app.cursor_position += c.len_utf8();
                    }
                    KeyCode::Backspace => {
                        if app.cursor_position > 0 {
                            if let Some((i, _c)) = app
                                .input
                                .char_indices()
                                .filter(|&(i, _)| i < app.cursor_position)
                                .last()
                            {
                                app.input.remove(i);
                                app.cursor_position = i;
                            }
                        }
                    }
                    KeyCode::Delete => {
                        if app.cursor_position < app.input.len() {
                            app.input.remove(app.cursor_position);
                        }
                    }
                    KeyCode::End => app.cursor_position = app.input.len(),
                    KeyCode::Home => app.cursor_position = 0,
                    KeyCode::PageUp => app.scroll_offset = app.scroll_offset.saturating_sub(5),
                    KeyCode::PageDown => app.scroll_offset = app.scroll_offset.saturating_add(5),
                    KeyCode::Up => app.scroll_offset = app.scroll_offset.saturating_sub(1),
                    KeyCode::Down => app.scroll_offset = app.scroll_offset.saturating_add(1),
                    KeyCode::Enter if !app.input.is_empty() => {
                        let cmd = app.input.trim().to_string();
                        app.messages.push(Message::user(vec![cmd.clone().into()]));
                        let _ = app.tx.send(cmd);
                        app.input.clear();
                        app.cursor_position = 0;
                        app.scroll_offset = u16::MAX;
                    }

                    _ => {}
                },
                // mouse events:
                Event::Mouse(mouse_event) => match mouse_event.kind {
                    event::MouseEventKind::ScrollUp => {
                        app.scroll_offset = app.scroll_offset.saturating_sub(3);
                    }
                    event::MouseEventKind::ScrollDown => {
                        app.scroll_offset = app.scroll_offset.saturating_add(3);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

/// Renderers the user interface
fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // header
            Constraint::Min(1),    // chat
            Constraint::Length(3), // input
            Constraint::Length(1), // footer
        ])
        .split(area);

    render_header(f, chunks[0]);

    let chat_area = chunks[1];
    let max_width = chat_area.width.saturating_sub(2) as usize;

    let mut history: Vec<Line> = Vec::new();

    for msg in &app.messages {
        let (role_name, color) = match msg.role {
            Role::User => (" USER ", Color::Cyan),
            Role::Assistant => (" OVSY ", Color::Cyan),
            _ => (" SYSTEM ", Color::Cyan),
        };

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
            .collect::<String>();

        // parsing a message:
        let mut msg_lines = parse_markdown(&text_content, max_width);

        // add a message role prefix:
        if let Some(first_line) = msg_lines.get_mut(0) {
            let mut new_spans = vec![
                Span::styled(
                    role_name,
                    Style::default().bg(color).fg(Color::Black).bold(),
                ),
                Span::raw(" "),
            ];
            new_spans.extend(first_line.spans.clone());
            *first_line = Line::from(new_spans);
        }

        history.extend(msg_lines);

        // add spacing between messages:
        if msg.role.is_assistant() {
            history.push(Line::raw(""));
        }
    }

    let chat_height = chat_area.height.saturating_sub(2) as usize;
    let total_lines = history.len();
    let max_scroll = total_lines.saturating_sub(chat_height) as u16;

    if app.scroll_offset > max_scroll {
        app.scroll_offset = max_scroll;
    }

    // calculate scroll lines:
    let current_line = if total_lines <= chat_height {
        total_lines
    } else {
        (app.scroll_offset as usize + chat_height).min(total_lines)
    };

    f.render_widget(
        Paragraph::new(history)
            .scroll((app.scroll_offset, 0))
            .block(
                Block::default()
                    .borders(Borders::LEFT)
                    .border_style(Style::default().dim())
                    .title(format!(
                        " [Lines: {}/{}] ",
                        if current_line > 0 {
                            current_line - 1
                        } else {
                            current_line
                        },
                        if total_lines > 0 {
                            total_lines - 1
                        } else {
                            total_lines
                        }
                    )),
            ),
        chat_area,
    );

    // --- INPUT BLOCK ---
    let input_area = chunks[2];
    let input_style = if app.is_thinking {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Cyan)
    };
    let input_title = if app.is_thinking {
        " Thinking... "
    } else {
        " Input "
    };

    f.render_widget(
        Paragraph::new(format!("  → {}", app.input)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(Span::styled(input_title, Style::default().bold()))
                .border_style(input_style),
        ),
        input_area,
    );

    // --- CURSOR POSITIONING ---
    // x: the beginning of the block + prefix indentation (4) + cursor position in the line
    // y: the beginning of the block + 1 (offset from the upper border of the frame)
    if !app.is_thinking {
        use unicode_width::UnicodeWidthStr;

        let text_before_cursor = &app.input[..app.cursor_position];
        let visual_width = text_before_cursor.width();

        let x_offset = input_area.x + 1 + 4;

        f.set_cursor_position((x_offset + (visual_width as u16), input_area.y + 1));
    }

    // --- FOOTER ---
    let help = Line::from(vec![
        " /clear ".bold().cyan(),
        "Clean context ".dim(),
        " /compact ".bold().cyan(),
        "Shrink context ".dim(),
        " /summary ".bold().cyan(),
        "Summarize talk ".dim(),
        " ESC ".bold().red(),
        "Exit ".dim(),
    ]);
    f.render_widget(Paragraph::new(help), chunks[3]);
}

/// Renders the header
fn render_header(f: &mut ratatui::Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" Ovsy Assistant {} ", app_version()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // vertical layout:
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    // gorizontal layout:
    let content_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(2),      // left gap
            Constraint::Percentage(40), // left info block
            Constraint::Percentage(60), // right info block
            Constraint::Length(2),      // right gap
        ])
        .split(layout[1]);

    // get current directory:
    let current_path = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "Unknown".to_string());

    // left info block:
    let left_text = Text::from(vec![
        Line::from(vec![
            Span::styled("Model: ", Style::default().white().bold()),
            Span::styled(
                Settings::get().assistant.completions.model.clone(),
                Style::default().gray(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Directory: ", Style::default().white().bold()),
            Span::styled(current_path, Style::default().gray()),
        ]),
    ]);

    // right info block:
    let right_text = Text::from(
        vec![
            Line::from("Digital liberation through local-first orchestration."),
            Line::from("Reclaim your data, build your own intelligence."),
        ]
        .iter()
        .map(|l| Line::from(l.to_string().dim()))
        .collect::<Vec<_>>(),
    );

    f.render_widget(Paragraph::new(left_text), content_area[1]);
    f.render_widget(
        Paragraph::new(right_text).alignment(ratatui::layout::Alignment::Right),
        content_area[2],
    );
}

//        MARKDOWN:

/// The parser state
struct ParserState {
    in_code_block: bool,
    is_bold: bool,
    is_italic: bool,
    is_inline_code: bool,
    is_crossed: bool,
}

/// Parses the markdown format
fn parse_markdown(text: &str, max_width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut state = ParserState {
        in_code_block: false,
        is_bold: false,
        is_italic: false,
        is_inline_code: false,
        is_crossed: false,
    };

    for raw_line in text.lines() {
        // --- CODE-BLOCKS ---
        if raw_line.starts_with("```") {
            state.in_code_block = !state.in_code_block;

            if state.in_code_block {
                // parse lang name (as example, "rust" from "```rust")
                let lang = raw_line.trim_start_matches('`').trim();
                let display_lang = if lang.is_empty() { "code" } else { lang };

                // draw lang name:
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

        if state.in_code_block {
            let wrapped = textwrap::wrap(raw_line, max_width);
            if wrapped.is_empty() {
                // draw empty lines also:
                lines.push(Line::from(vec![Span::styled(
                    " ",
                    Style::default().bg(Color::Rgb(45, 45, 45)),
                )]));
            } else {
                for part in wrapped {
                    lines.push(Line::from(vec![Span::styled(
                        part.into_owned(),
                        Style::default().bg(Color::Rgb(45, 45, 45)).fg(Color::White),
                    )]));
                }
            }
            continue;
        }

        // --- GORIZONTAL LINE ---
        if raw_line.starts_with("---") {
            lines.push(Line::from(vec![Span::styled(
                "─".repeat(max_width),
                Style::default().fg(Color::DarkGray),
            )]));
            continue;
        }

        // --- HEADERS, LISTS & QUOTES ---
        let is_quote = raw_line.starts_with("> ");
        let is_unordered_list = raw_line.starts_with("- ");
        let is_ordered_list = raw_line
            .chars()
            .next()
            .map_or(false, |c| c.is_ascii_digit())
            && raw_line.contains(". ");

        let (content, base_style, prefix) = if raw_line.starts_with("# ") {
            (
                raw_line.trim_start_matches("#").trim(),
                Style::default().fg(Color::Cyan).bold().underlined(),
                None,
            )
        } else if raw_line.starts_with("## ") || raw_line.starts_with("### ") {
            (
                raw_line.trim_start_matches("#").trim(),
                Style::default().fg(Color::Cyan).bold(),
                None,
            )
        } else if raw_line.starts_with("#### ") {
            (
                raw_line.trim_start_matches("####").trim(),
                Style::default().fg(Color::Cyan).bold().italic(),
                None,
            )
        } else if raw_line.starts_with("##### ") || raw_line.starts_with("###### ") {
            (
                raw_line.trim_start_matches("#").trim(),
                Style::default().fg(Color::Cyan).italic(),
                None,
            )
        } else if is_quote {
            (
                raw_line.trim_start_matches("> ").trim(),
                Style::default()
                    .bg(Color::Rgb(45, 45, 45))
                    .fg(Color::Rgb(200, 200, 200))
                    .italic(),
                None,
            )
        } else if is_unordered_list {
            (
                raw_line.trim_start_matches("- ").trim(),
                Style::default().fg(Color::White),
                Some(Span::styled(" • ", Style::default().fg(Color::Cyan).bold())),
            )
        } else if is_ordered_list {
            let parts: Vec<&str> = raw_line.splitn(2, ". ").collect();
            (
                parts.get(1).unwrap_or(&"").trim(),
                Style::default().fg(Color::White),
                Some(Span::styled(
                    format!(" {}. ", parts[0]),
                    Style::default().fg(Color::Cyan).bold(),
                )),
            )
        } else {
            (raw_line, Style::default().fg(Color::White), None)
        };

        // --- INLINE PARSING ---
        let wrapped_lines = textwrap::wrap(content, max_width);
        for wrapped_row in wrapped_lines {
            let mut spans = Vec::new();
            if let Some(ref p) = prefix {
                spans.push(p.clone());
            }

            let mut current_text = String::new();
            let owned_row = wrapped_row.into_owned();
            let mut chars = owned_row.chars().peekable();

            while let Some(c) = chars.next() {
                match c {
                    '[' if !state.is_inline_code => {
                        if !current_text.is_empty() {
                            push_text_with_urls(
                                &mut spans,
                                &current_text,
                                get_style(&state, base_style),
                            );
                            current_text.clear();
                        }

                        let mut link_text = String::new();
                        while let Some(&next_c) = chars.peek() {
                            if next_c == ']' {
                                chars.next();
                                break;
                            }
                            link_text.push(chars.next().unwrap());
                        }

                        if chars.peek() == Some(&'(') {
                            chars.next();
                            let mut url = String::new();
                            while let Some(&next_c) = chars.peek() {
                                if next_c == ')' {
                                    chars.next();
                                    break;
                                }
                                url.push(chars.next().unwrap());
                            }

                            let is_duplicate = url.contains(&link_text)
                                || link_text
                                    .contains(&url.replace("https://", "").replace("http://", ""));

                            if is_duplicate {
                                spans.push(Span::styled(
                                    url,
                                    get_style(&state, base_style).fg(Color::Cyan).underlined(),
                                ));
                            } else {
                                spans.push(Span::styled(link_text, get_style(&state, base_style)));
                                spans.push(Span::styled(" (", get_style(&state, base_style)));
                                spans.push(Span::styled(
                                    url,
                                    get_style(&state, base_style).fg(Color::Cyan).underlined(),
                                ));
                                spans.push(Span::styled(")", get_style(&state, base_style)));
                            }
                        } else {
                            let is_url = link_text.starts_with("http");
                            let style = if is_url {
                                get_style(&state, base_style).fg(Color::Cyan).underlined()
                            } else {
                                get_style(&state, base_style).fg(Color::Cyan)
                            };

                            let display = if is_url {
                                link_text
                            } else {
                                format!("[{}]", link_text)
                            };
                            spans.push(Span::styled(display, style));
                        }
                    }
                    '~' if chars.peek() == Some(&'~') => {
                        chars.next();
                        if !current_text.is_empty() {
                            spans.push(Span::styled(
                                current_text.clone(),
                                get_style(&state, base_style),
                            ));
                            current_text.clear();
                        }
                    }
                    '`' => {
                        if !current_text.is_empty() {
                            spans.push(Span::styled(
                                current_text.clone(),
                                get_style(&state, base_style),
                            ));
                            current_text.clear();
                        }
                        state.is_inline_code = !state.is_inline_code;
                    }
                    '*' => {
                        let is_double = chars.peek() == Some(&'*');
                        if is_double {
                            chars.next();
                        }
                        if !current_text.is_empty() {
                            spans.push(Span::styled(
                                current_text.clone(),
                                get_style(&state, base_style),
                            ));
                            current_text.clear();
                        }
                        if is_double {
                            state.is_bold = !state.is_bold;
                        } else {
                            state.is_italic = !state.is_italic;
                        }
                    }
                    _ => current_text.push(c),
                }
            }
            if !current_text.is_empty() {
                push_text_with_urls(&mut spans, &current_text, get_style(&state, base_style));
            }
            lines.push(Line::from(spans))
        }
    }
    lines
}

// Helper function for style
fn get_style(state: &ParserState, base: Style) -> Style {
    let mut s = base;

    // crossed text:
    if state.is_crossed {
        s = s.crossed_out();
    }

    // inline code:
    if state.is_inline_code {
        s = s.fg(Color::Cyan).bg(Color::Rgb(40, 40, 40));
    } else {
        if state.is_bold {
            s = s.bold();
        }
        if state.is_italic {
            s = s.italic();
        }
    }
    s
}

/// Push a text with URL links
fn push_text_with_urls(spans: &mut Vec<Span<'static>>, text: &str, base_style: Style) {
    let words = text.split_inclusive(|c: char| c.is_whitespace());

    for word in words {
        let trimmed = word.trim();
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            spans.push(Span::styled(
                word.to_string(),
                base_style.fg(Color::Cyan).underlined(),
            ));
        } else {
            spans.push(Span::styled(word.to_string(), base_style));
        }
    }
}
