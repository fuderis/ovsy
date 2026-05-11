use crate::prelude::*;
use anylm::{Content, Message};
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
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};
use std::{io, process::Command};
use tokio::{
    sync::mpsc,
    time::{self, Duration},
};
use unicode_width::UnicodeWidthStr;

/// The app state
struct AppState {
    input: String,
    cursor_position: usize,
    messages: Arc<State<(Vec<Message>, usize)>>,
    is_thinking: bool,
    scroll_offset: u16,
    input_scroll_offset: u16,
    tx: mpsc::UnboundedSender<String>,
    tick_count: u64,
    commands: Vec<(&'static str, &'static str)>,
    is_busy: Arc<Flag>,
    chat_area: ratatui::layout::Rect,
    input_area: ratatui::layout::Rect,
}

impl AppState {
    pub fn new(tx: mpsc::UnboundedSender<String>) -> Self {
        Self {
            input: String::new(),
            cursor_position: 0,
            messages: arc!(State::new()),
            is_thinking: false,
            scroll_offset: 0,
            input_scroll_offset: 0,
            tx,
            tick_count: 0,
            commands: vec![
                ("/clear", "Clear the dialog context"),
                ("/compact", "Compress the dialog context"),
                ("/exit", "Exit the assistant"),
            ],
            is_busy: arc!(Flag::new()),
            chat_area: Default::default(),
            input_area: Default::default(),
        }
    }

    async fn handle_input(&mut self) {
        let trimmed = self.input.trim();
        if trimmed.is_empty() {
            return;
        }

        if trimmed.starts_with('/') {
            match trimmed {
                "/exit" => {}
                _ => {
                    let _ = self.tx.send(trimmed.to_string());
                }
            }
        } else {
            {
                let mut msgs = self.messages.lock().await;
                msgs.0.push(Message::user(vec![trimmed.to_string().into()]));
                msgs.1 = AppState::calc_tokens(&msgs.0);
            }

            // send query to worker (it won't work without it):
            let _ = self.tx.send(trimmed.to_string());
        }

        self.input.clear();
        self.cursor_position = 0;
        self.input_scroll_offset = 0;
        self.scroll_offset = u16::MAX;
    }

    pub fn calc_tokens(msgs: &Vec<Message>) -> usize {
        msgs.iter().map(|m| m.tokens_count).sum()
    }
}

/// Handles the `chat` command
pub async fn handle() -> Result<()> {
    let port = Settings::get().server.port;
    let client = reqwest::Client::new();

    // check the server:
    let status_url = str!("http://127.0.0.1:{port}/update");
    if client.get(&status_url).send().await.is_err() {
        // starting the server:
        let bin_path = path!("$/ovsy-server{}", if cfg!(windows) { "exe" } else { "" });
        let spawn_res = Command::new(bin_path).arg("start").spawn();

        match spawn_res {
            Ok(_) => {
                // wait server initializing:
                let mut started = false;
                for _ in 0..10 {
                    time::sleep(Duration::from_millis(500)).await;
                    if client.get(&status_url).send().await.is_ok() {
                        started = true;
                        break;
                    }
                }

                if !started {
                    eprintln!(
                        "{}: Server started but is not responding.",
                        "Timeout".red().bold()
                    );
                    return Ok(());
                }
            }
            Err(e) => {
                eprintln!("{}: Failed to execute server: {e}", "Error".red().bold());
                return Ok(());
            }
        }
    }

    let (input_tx, input_rx) = mpsc::unbounded_channel::<String>();
    let (ui_tx, mut ui_rx) = mpsc::unbounded_channel::<Chunk>();
    let mut app = AppState::new(input_tx);

    // spawn network worker:
    tokio::spawn(chat_worker(
        input_rx,
        ui_tx,
        app.messages.clone(),
        app.is_busy.clone(),
    ));

    // setup user interface:
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))?;

    let res = run_app(&mut terminal, &mut app, &mut ui_rx).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    res
}

/// A worker for networking
async fn chat_worker(
    mut input_rx: mpsc::UnboundedReceiver<String>,
    ui_tx: mpsc::UnboundedSender<Chunk>,
    messages: Arc<State<(Vec<Message>, usize)>>,
    is_busy: Arc<Flag>,
) {
    let port = Settings::get().server.port;
    let client = reqwest::Client::new();
    let base_url = str!("http://127.0.0.1:{port}");

    while let Some(input) = input_rx.recv().await {
        is_busy.set(true);

        // commands handler:
        let trimmed = input.trim();
        if trimmed.starts_with("/") {
            match trimmed.to_lowercase().as_ref() {
                // Clear context:
                "/clear" | "/clean" => {
                    messages.set((vec![], 0)).await;
                    is_busy.set(false);
                    continue;
                }

                // Compress context:
                "/compact" | "/compress" => {
                    let _ = ui_tx.send(Chunk::Think {
                        think: "⚙ Compressing context...".into(),
                    });

                    let messages_count = messages.get().await.0.len();
                    let max_messages = Settings::get().assistant.max_messages * 2;
                    let compress_count = (messages_count.saturating_sub(max_messages) / 2) * 2;

                    if messages_count > 2 && compress_count > 0 {
                        let to_compress = messages.get().await.0[..compress_count].to_vec();

                        let res = client
                            .post(str!("{base_url}/compact"))
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
                                let preserved_msgs =
                                    messages.get().await.0.clone()[compress_count..].to_vec();

                                let mut messages_temp = vec![Message::assistant(vec![
                                    str!("**Summarized**: {}\n---\n", summary.trim()).into(),
                                ])];
                                messages_temp.extend(preserved_msgs);

                                let tokens_count = AppState::calc_tokens(&messages_temp);
                                messages.set((messages_temp, tokens_count)).await;
                            }
                        }
                    }

                    is_busy.set(false);
                    continue;
                }

                _ => {
                    is_busy.set(false);
                    continue;
                }
            }
        }

        let res = client
            .post(str!("{base_url}/handle"))
            .json(&UserQuery {
                user_id: 0,
                messages: (*messages.get().await).0.clone(),
            })
            .send()
            .await;

        match res {
            Ok(response) => {
                let mut full_answer = String::new();
                let mut stream =
                    Stream::read::<Chunk>(response.bytes_stream().map(|c| c.map_err(Into::into)));

                while let Ok(Some(chunk)) = stream.read().await {
                    if let Chunk::Answer { ref answer } = chunk {
                        full_answer.push_str(answer);
                    }
                    let _ = ui_tx.send(chunk);
                }
            }
            Err(e) => {
                let _ = ui_tx.send(Chunk::Error {
                    error: str!("Connection error: {}", e),
                });
            }
        }

        is_busy.set(false);
    }
}

/// Runs the terminal app
async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut AppState,
    ui_rx: &mut mpsc::UnboundedReceiver<Chunk>,
) -> Result<()> {
    // enabling mouse capture (usually done in main, but duplicating the logic):
    execute!(std::io::stdout(), event::EnableMouseCapture)?;

    loop {
        app.tick_count += 1;
        terminal.draw(|f| ui(f, app)).map_err(|e| e.to_string())?;

        // 1. Processing incoming data from the server:
        if let Ok(chunk) = ui_rx.try_recv() {
            match chunk {
                Chunk::Think { .. } => app.is_thinking = true,
                Chunk::Answer { answer } => {
                    app.is_thinking = false;
                    let mut msgs = app.messages.lock().await;

                    match msgs.0.last_mut() {
                        Some(msg) if msg.role.is_assistant() => {
                            msg.map(|cnt| {
                                if let Some(Content::Text { text }) = cnt.get_mut(0) {
                                    text.push_str(&answer);
                                }
                            });
                        }

                        _ => {
                            msgs.0.push(Message::assistant(vec![answer.into()]));
                        }
                    }
                    msgs.1 = AppState::calc_tokens(&msgs.0);

                    // scroll down when receiving new tokens:
                    app.scroll_offset = u16::MAX;
                }
                Chunk::Error { error } => {
                    app.is_thinking = false;
                    let mut msgs = app.messages.lock().await;

                    msgs.0
                        .push(Message::system(vec![str!("Error: {error}").into()]));
                    msgs.1 = AppState::calc_tokens(&msgs.0);
                }
                _ => {}
            }
        }

        // 2. Event Handling (Keyboard and Mouse):
        if event::poll(std::time::Duration::from_millis(16))? {
            match event::read()? {
                // Mouse handling:
                Event::Mouse(mouse_event) => match mouse_event.kind {
                    event::MouseEventKind::ScrollUp => {
                        if app
                            .chat_area
                            .contains((mouse_event.column, mouse_event.row).into())
                        {
                            app.scroll_offset = app.scroll_offset.saturating_sub(2);
                        } else if app
                            .input_area
                            .contains((mouse_event.column, mouse_event.row).into())
                        {
                            app.input_scroll_offset = app.input_scroll_offset.saturating_sub(1);
                        }
                    }
                    event::MouseEventKind::ScrollDown => {
                        if app
                            .chat_area
                            .contains((mouse_event.column, mouse_event.row).into())
                        {
                            app.scroll_offset = app.scroll_offset.saturating_add(2);
                        } else if app
                            .input_area
                            .contains((mouse_event.column, mouse_event.row).into())
                        {
                            app.input_scroll_offset = app.input_scroll_offset.saturating_add(1);
                        }
                    }
                    _ => {}
                },

                // Key handling:
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    let has_shift = key.modifiers.contains(event::KeyModifiers::SHIFT);

                    match key.code {
                        KeyCode::Esc => return Ok(()),
                        KeyCode::Enter => {
                            if !app.is_busy.get() {
                                if app.input.trim() == "/exit" {
                                    return Ok(());
                                }
                                app.handle_input().await;
                            }
                        }
                        KeyCode::Char(c) => {
                            app.input.insert(app.cursor_position, c);
                            app.cursor_position += c.len_utf8();
                        }
                        KeyCode::Backspace => {
                            if app.cursor_position > 0 {
                                if let Some((i, _)) = app
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
                        // Scroll up/down
                        KeyCode::Up => app.scroll_offset = app.scroll_offset.saturating_sub(1),
                        KeyCode::Down => app.scroll_offset = app.scroll_offset.saturating_add(1),

                        // PageUp / PageDown with Shift modifier
                        KeyCode::PageUp => {
                            if has_shift {
                                app.input_scroll_offset = app.input_scroll_offset.saturating_sub(5);
                            } else {
                                app.scroll_offset = app.scroll_offset.saturating_sub(10);
                            }
                        }
                        KeyCode::PageDown => {
                            if has_shift {
                                app.input_scroll_offset = app.input_scroll_offset.saturating_add(5);
                            } else {
                                app.scroll_offset = app.scroll_offset.saturating_add(10);
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
}

/// Renderers the user interface
fn ui(f: &mut ratatui::Frame, app: &mut AppState) {
    let area = f.area();

    // --- 1. CALCULATE INPUT HEIGHT ---
    let prefix = "  → ";
    let inner_width = area.width.saturating_sub(2) as usize;

    let wrap_options = textwrap::Options::new(inner_width)
        .break_words(true)
        .word_separator(textwrap::WordSeparator::AsciiSpace);

    let full_text = str!("{}{}", prefix, app.input);
    let wrapped_lines = textwrap::wrap(&full_text, wrap_options.clone());

    let line_count = wrapped_lines.len().max(1) as u16;
    let dynamic_input_height = (line_count + 2).clamp(3, 8);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(1),
            Constraint::Length(dynamic_input_height),
            Constraint::Length(1),
        ])
        .split(area);

    app.chat_area = chunks[1];
    app.input_area = chunks[2];

    // --- 3. HEADER ---
    render_header(f, chunks[0]);

    // --- 4. CHAT HISTORY ---
    let chat_area = chunks[1];
    let chat_inner_height = chat_area.height.saturating_sub(2);
    let chat_inner_width = chat_area.width.saturating_sub(1);

    let mut history: Vec<Line> = Vec::new();

    for msg in app.messages.dirty_get().0.iter() {
        // metadata line:
        let meta_line = Line::from(vec![if msg.role.is_user() {
            Span::styled(
                str!(
                    "[{}]",
                    msg.timestamp
                        .map(|t| t
                            .with_timezone(&chrono::Local)
                            .format("%Y-%m-%dT%H:%M:%S")
                            .to_string())
                        .unwrap_or_else(|| "??:??:??".to_string())
                ),
                Style::default().bg(Color::Cyan).fg(Color::Black).bold(),
            )
        } else {
            Span::styled(str!(), Style::default())
        }]);
        history.push(meta_line);

        // message content:
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

        let msg_lines = parse_markdown(&text_content, chat_inner_width as usize);

        if msg.role.is_user() {
            for line in msg_lines {
                history.push(
                    line.patch_style(
                        Style::default()
                            .bg(Color::Rgb(20, 20, 20))
                            .fg(Color::DarkGray),
                    ),
                );
            }
        } else {
            history.extend(msg_lines);
        }

        if msg.role.is_assistant() {
            history.push(Line::raw(""));
        }
    }

    let total_lines = history.len() as u16;
    let chat_height = chat_area.height.saturating_sub(2) as u16;
    let max_chat_scroll = total_lines.saturating_sub(chat_inner_height);
    app.scroll_offset = app.scroll_offset.min(max_chat_scroll);

    // calculate scroll lines:
    let current_line = if total_lines <= chat_height {
        total_lines
    } else {
        (app.scroll_offset + chat_height).min(total_lines)
    };

    let max_tokens = Settings::get()
        .assistant
        .completions
        .max_tokens
        .unwrap_or(1) as usize;

    let current_tokens = app.messages.dirty_get().1;

    fn make_progress_bar(current: usize, max: usize, width: usize) -> String {
        if max == 0 {
            return " ".repeat(width);
        }
        let ratio = (current as f32 / max as f32).clamp(0.0, 1.0);
        let filled_len = (ratio * width as f32).round() as usize;
        let empty_len = width.saturating_sub(filled_len);

        str!("{}{}", "■".repeat(filled_len), "□".repeat(empty_len))
    }

    let pb = make_progress_bar(current_tokens, max_tokens, 10);
    let tokens_text = str!("{}/{}", current_tokens, max_tokens);
    let lines_text = str!(
        "{}/{}",
        current_line.saturating_sub(1),
        total_lines.saturating_sub(1)
    );

    let ratio = if max_tokens > 0 {
        (current_tokens as f32 / max_tokens as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let pb_color = if ratio <= 0.7 {
        Color::Cyan
    } else if ratio < 0.9 {
        Color::Yellow
    } else {
        Color::Red
    };

    let title_left = Line::from(vec![
        Span::raw(" ["),
        Span::styled(pb, Style::default().fg(pb_color)),
        Span::raw("] "),
        Span::styled(tokens_text, Style::default().fg(Color::White)),
        Span::raw(" "),
    ]);

    let title_right = Line::from(vec![
        Span::raw(" "),
        Span::styled(lines_text, Style::default().fg(Color::Gray)),
        Span::raw(" "),
    ]);

    let title_right2 = Line::from(vec![Span::raw(" ")]);

    f.render_widget(
        Paragraph::new(history)
            .scroll((app.scroll_offset, 0))
            .block(
                Block::default()
                    .borders(Borders::LEFT | Borders::TOP)
                    .border_style(Style::default().dim())
                    .title(title_left)
                    .title(title_right.alignment(ratatui::layout::HorizontalAlignment::Right))
                    .title(title_right2.alignment(ratatui::layout::HorizontalAlignment::Right)),
            ),
        chat_area,
    );

    // --- 5. INPUT BLOCK ---
    let input_area = chunks[2];
    let input_inner_height = input_area.height.saturating_sub(2);

    // auto-scroll:
    let text_before_cursor = &app.input[..app.cursor_position];
    let text_temp = str!("{}{}", prefix, text_before_cursor);
    let wrapped_before = textwrap::wrap(&text_temp, wrap_options.clone());
    let cursor_row = (wrapped_before.len() as u16).saturating_sub(1);

    // if cursor is below the visible area:
    if cursor_row >= app.input_scroll_offset + input_inner_height {
        app.input_scroll_offset = cursor_row - input_inner_height + 1;
    }
    // if cursor is above the visible area (for example, after delete lines):
    if cursor_row < app.input_scroll_offset {
        app.input_scroll_offset = cursor_row;
    }

    let input_area = chunks[2];
    let dots_count = (app.tick_count / 30) % 4;
    let dots = ".".repeat(dots_count as usize);
    let thinking_text = str!(" Thinking{dots:<3} ");

    f.render_widget(
        Paragraph::new(str!("{}{}", prefix, app.input))
            .wrap(Wrap { trim: false })
            .scroll((app.input_scroll_offset, 0))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(if app.is_busy.get() {
                        Span::styled(thinking_text, Style::default().fg(Color::White).italic())
                    } else {
                        Span::styled(" Input ", Style::default().fg(Color::Cyan))
                    })
                    .border_style(Style::default().fg(if app.is_busy.get() {
                        Color::White
                    } else {
                        Color::Cyan
                    })),
            ),
        input_area,
    );

    // --- 6. CURSOR POSITIONING ---
    if !app.is_thinking {
        let last_line = wrapped_before
            .last()
            .map(|s| s.chars().count())
            .unwrap_or(0);

        let trimmed_input = app.input.trim();
        let add_x = if trimmed_input.is_empty() {
            app.input = trimmed_input.to_owned();
            app.cursor_position = 0;
            1
        } else if app.input.ends_with(" ") {
            (app.input.chars().count() - trimmed_input.chars().count()) as u16
        } else {
            0
        };

        f.set_cursor_position((
            input_area.x + 1 + add_x + last_line as u16,
            input_area.y + 1 + cursor_row - app.input_scroll_offset,
        ));
    }

    // --- 7. FOOTER ---
    render_footer(f, chunks[3], app);
}

/// Renders the footer section
fn render_footer(f: &mut ratatui::Frame, area: Rect, app: &AppState) {
    let commands = &app.commands;

    let index = (app.tick_count / 360) as usize % commands.len();
    let (cmd, desc) = commands[index];

    let help_line = Line::from(vec![
        " ? ".dim(),
        cmd.bold().cyan(),
        " ".into(),
        desc.gray().italic(),
    ]);

    f.render_widget(
        Paragraph::new(help_line).alignment(ratatui::layout::Alignment::Left),
        area,
    );
}

/// Renders the header section
fn render_header(f: &mut ratatui::Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(str!(" Ovsy Assistant {} ", app_version()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // vertical layout:
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3)])
        .split(inner);

    // gorizontal layout:
    let content_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(2),       // left gap
            Constraint::Percentage(100), // info block
            Constraint::Length(2),       // right gap
        ])
        .split(layout[0]);

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
            Span::styled("Dir: ", Style::default().white().bold()),
            Span::styled(current_path, Style::default().gray()),
        ]),
    ]);

    f.render_widget(Paragraph::new(left_text), content_area[1]);
}

//        MARKDOWN:

/// The parser state
struct ParserState {
    in_code_block: bool,
    in_table: bool,
    table_rows: Vec<Vec<String>>,
}

/// Parses the markdown format
fn parse_markdown(text: &str, max_width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut state = ParserState {
        in_code_block: false,
        in_table: false,
        table_rows: vec![],
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
                    str!(" {} ", display_lang.to_lowercase()),
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

        // --- NEW LINE <br> ---
        if raw_line.contains("<br>") || raw_line.contains("<br/>") {
            lines.push(Line::from(" "));
            continue;
        }

        // --- TABLES ---
        let is_table_row = raw_line.trim().starts_with('|');
        if is_table_row {
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
            render_collected_table(&mut lines, &mut state, max_width);
            state.in_table = false;
            state.table_rows.clear();
        }

        // --- HORIZONTAL LINES ---
        if raw_line.starts_with("---") {
            lines.push(Line::from(vec![Span::styled(
                "─".repeat(max_width),
                Style::default().fg(Color::DarkGray),
            )]));
            continue;
        }

        // --- HEADERS, LISTS & QUOTES ---
        let (content, base_style, prefix) = if raw_line.starts_with("# ") {
            (
                raw_line.trim_start_matches("# ").trim(),
                Style::default().fg(Color::Cyan).bold().underlined(),
                None,
            )
        } else if raw_line.starts_with("## ") {
            (
                raw_line.trim_start_matches("## ").trim(),
                Style::default().fg(Color::Cyan).bold(),
                None,
            )
        } else if raw_line.starts_with("### ") {
            (
                raw_line.trim_start_matches("### ").trim(),
                Style::default().fg(Color::Cyan).bold(),
                None,
            )
        } else if raw_line.starts_with("#### ") {
            (
                raw_line.trim_start_matches("#### ").trim(),
                Style::default().fg(Color::White).bold(),
                None,
            )
        } else if raw_line.starts_with("##### ") {
            (
                raw_line.trim_start_matches("##### ").trim(),
                Style::default().fg(Color::White).bold().italic(),
                None,
            )
        } else if raw_line.starts_with("###### ") {
            (
                raw_line.trim_start_matches("###### ").trim(),
                Style::default().fg(Color::White).italic(),
                None,
            )
        } else if raw_line.starts_with("- ") {
            (
                raw_line.trim_start_matches("- ").trim(),
                Style::default().fg(Color::White),
                Some(Span::styled(" • ", Style::default().fg(Color::Cyan).bold())),
            )
        } else if raw_line.starts_with("> ") {
            (
                raw_line.trim_start_matches("> ").trim(),
                Style::default()
                    .bg(Color::Rgb(45, 45, 45))
                    .fg(Color::Rgb(200, 200, 200))
                    .italic(),
                None,
            )
        } else {
            (raw_line, Style::default().fg(Color::White), None)
        };

        let wrapped_lines = textwrap::wrap(content, max_width);
        for wrapped_row in wrapped_lines {
            let mut spans = Vec::new();
            if let Some(ref p) = prefix {
                spans.push(p.clone());
            }
            spans.extend(parse_inline_styles(&wrapped_row, base_style));
            lines.push(Line::from(spans));
        }
    }

    if state.in_table {
        render_collected_table(&mut lines, &mut state, max_width);
    }
    lines
}

/// Renders the table
fn render_collected_table(lines: &mut Vec<Line>, state: &mut ParserState, max_width: usize) {
    if state.table_rows.is_empty() {
        return;
    }

    let col_count = state.table_rows[0].len();
    let mut ideal_widths = vec![0; col_count];

    // calculate width:
    for row in &state.table_rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count {
                ideal_widths[i] = ideal_widths[i].max(strip_markdown(cell).width());
            }
        }
    }

    let borders_overhead = 2 + (col_count * 3);
    let available_width = max_width.saturating_sub(borders_overhead);
    let total_ideal_width: usize = ideal_widths.iter().sum();

    let mut final_widths = ideal_widths.clone();
    if total_ideal_width > available_width && available_width > 0 {
        for i in 0..col_count {
            final_widths[i] = (ideal_widths[i] * available_width) / total_ideal_width;
            if final_widths[i] == 0 {
                final_widths[i] = 1;
            }
        }
    }

    // draw borders:
    let mut top_line = vec![Span::styled("┌", Style::default().dim())];
    for (i, &w) in final_widths.iter().enumerate() {
        top_line.push(Span::styled("─".repeat(w + 2), Style::default().dim()));
        if i < col_count - 1 {
            top_line.push(Span::styled("┬", Style::default().dim()));
        }
    }
    top_line.push(Span::styled("┐", Style::default().dim()));
    lines.push(Line::from(top_line));

    let rows_len = state.table_rows.len();
    for (r_idx, row) in state.table_rows.iter().enumerate() {
        let mut wrapped_cells: Vec<Vec<String>> = Vec::new();
        let mut max_cell_lines = 1;

        for (i, cell) in row.iter().enumerate() {
            if i >= col_count {
                break;
            }
            let clean_cell = strip_markdown(cell);
            let wrapped = textwrap::wrap(&clean_cell, final_widths[i]);
            let strings: Vec<String> = wrapped.into_iter().map(|s| s.into_owned()).collect();
            max_cell_lines = max_cell_lines.max(strings.len());
            wrapped_cells.push(strings);
        }

        for line_idx in 0..max_cell_lines {
            let mut line_spans = vec![Span::styled("│ ", Style::default().dim())];
            for (i, wrapped_lines) in wrapped_cells.iter().enumerate() {
                let content = wrapped_lines.get(line_idx).cloned().unwrap_or_default();
                let content_w = content.width();

                let base_style = if r_idx == 0 {
                    Style::default().cyan().bold()
                } else {
                    Style::default().white()
                };

                line_spans.extend(parse_inline_styles(&content, base_style));

                let padding = " ".repeat(final_widths[i].saturating_sub(content_w));
                line_spans.push(Span::styled(padding, base_style));
                line_spans.push(Span::styled(" │ ", Style::default().dim()));
            }
            lines.push(Line::from(line_spans));
        }

        // splitter:
        if r_idx < rows_len - 1 {
            let mut sep = vec![Span::styled("├", Style::default().dim())];
            let line_char = if r_idx == 0 { "━" } else { "─" };
            for (i, &w) in final_widths.iter().enumerate() {
                sep.push(Span::styled(
                    line_char.repeat(w + 2),
                    Style::default().dim(),
                ));
                if i < col_count - 1 {
                    sep.push(Span::styled("┼", Style::default().dim()));
                }
            }
            sep.push(Span::styled("┤", Style::default().dim()));
            lines.push(Line::from(sep));
        }
    }

    // bottom borders:
    let mut bottom_line = vec![Span::styled("└", Style::default().dim())];
    for (i, &w) in final_widths.iter().enumerate() {
        bottom_line.push(Span::styled("─".repeat(w + 2), Style::default().dim()));
        if i < col_count - 1 {
            bottom_line.push(Span::styled("┴", Style::default().dim()));
        }
    }
    bottom_line.push(Span::styled("┘", Style::default().dim()));
    lines.push(Line::from(bottom_line));
}

/// Inline style parsing (bold, italics, code, links)
fn parse_inline_styles(content: &str, base_style: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut current_text = String::new();
    let mut chars = content.chars().peekable();

    let mut is_bold = false;
    let mut is_italic = false;
    let mut is_inline_code = false;

    while let Some(c) = chars.next() {
        match c {
            '`' => {
                if !current_text.is_empty() {
                    spans.extend(push_text_with_urls(
                        &current_text,
                        get_active_style(base_style, is_bold, is_italic, is_inline_code),
                    ));
                    current_text.clear();
                }
                is_inline_code = !is_inline_code;
            }
            '*' => {
                let is_double = chars.peek() == Some(&'*');
                if is_double {
                    chars.next();
                }

                if !current_text.is_empty() {
                    spans.extend(push_text_with_urls(
                        &current_text,
                        get_active_style(base_style, is_bold, is_italic, is_inline_code),
                    ));
                    current_text.clear();
                }

                if is_double {
                    is_bold = !is_bold;
                } else {
                    is_italic = !is_italic;
                }
            }
            _ => current_text.push(c),
        }
    }

    if !current_text.is_empty() {
        spans.extend(push_text_with_urls(
            &current_text,
            get_active_style(base_style, is_bold, is_italic, is_inline_code),
        ));
    }

    spans
}

/// Helper function to return the text style
fn get_active_style(mut s: Style, bold: bool, italic: bool, code: bool) -> Style {
    if code {
        s = s.fg(Color::Cyan).bg(Color::Rgb(40, 40, 40));
    } else {
        if bold {
            s = s.bold();
        }
        if italic {
            s = s.italic();
        }
    }
    s
}

/// Push a text with URL links
fn push_text_with_urls(text: &str, base_style: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for word in text.split_inclusive(|c: char| c.is_whitespace()) {
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
    spans
}

/// Helper function for clearing text from markdown tags (for calculating width)
fn strip_markdown(text: &str) -> String {
    text.replace("**", "")
        .replace("__", "")
        .replace("*", "")
        .replace("_", "")
        .replace("`", "")
        .replace("<br>", "")
        .replace("<br/>", "")
}
