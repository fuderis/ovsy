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
    widgets::{Block, BorderType, Borders, Paragraph},
};
use std::io;
use tokio::sync::mpsc;
use unicode_width::UnicodeWidthStr;

/// The app state
struct AppState {
    input: String,
    cursor_position: usize,
    messages: Arc<State<Vec<Message>>>,
    is_thinking: bool,
    scroll_offset: u16,
    tx: mpsc::UnboundedSender<String>,
    tick_count: u64,
    commands: Vec<(&'static str, &'static str)>,
    is_busy: Arc<Flag>,
}

impl AppState {
    /// Creates a new app state
    pub fn new(tx: mpsc::UnboundedSender<String>) -> Self {
        Self {
            input: String::new(),
            cursor_position: 0,
            messages: arc!(State::new()),
            is_thinking: false,
            scroll_offset: 0,
            tx,
            tick_count: 0,
            commands: vec![
                ("/clear", "Clear the dialog context"),
                ("/compact", "Compress the dialog context"),
                ("/exit", "Exit the assistant"),
            ],
            is_busy: arc!(Flag::new()),
        }
    }

    /// Checks if the input is a command and processes it.
    async fn handle_input(&mut self) {
        let trimmed = self.input.trim();
        if trimmed.is_empty() {
            return;
        }

        // command:
        if trimmed.starts_with('/') {
            match trimmed {
                "/exit" => { /* the output is processed in the main loop via return */ }
                "/clear" => {
                    self.messages.set(vec![]).await;
                    let _ = self.tx.send(trimmed.to_string());
                }
                _ => {
                    let _ = self.tx.send(trimmed.to_string());
                }
            }
        }
        // message:
        else {
            self.messages
                .lock()
                .await
                .push(Message::user(vec![trimmed.to_string().into()]));
            let _ = self.tx.send(trimmed.to_string());
        }

        self.input.clear();
        self.cursor_position = 0;
        self.scroll_offset = u16::MAX;
    }
}

/// Handles the `chat` command
pub async fn handle() -> Result<()> {
    let port = Settings::get().server.port;
    let client = reqwest::Client::new();

    // checking the server:
    print!("🚀 Checking Ovsy server on port {}... ", port);

    let status_url = format!("http://127.0.0.1:{port}/status");

    if client.get(&status_url).send().await.is_err() {
        println!("{}\nStarting backend...", colored::Colorize::red("Offline"));

        // define the binary extension:
        let ext = if cfg!(windows) { "exe" } else { "" };
        let bin_path = path!("~/.ovsy/ovsy-server{ext}");

        // starting the server:
        let spawn_res = std::process::Command::new(bin_path).arg("start").spawn();

        match spawn_res {
            Ok(_) => {
                // wait server initializing:
                let mut started = false;
                for _ in 0..10 {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    if client.get(&status_url).send().await.is_ok() {
                        started = true;
                        break;
                    }
                }

                if !started {
                    eprintln!("❌ Timeout: Server started but is not responding.");
                    return Ok(());
                }
            }
            Err(e) => {
                eprintln!("❌ Failed to execute server binary: {e}");
                return Ok(());
            }
        }
    } else {
        println!("{}", colored::Colorize::green("Online"));
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
    messages: Arc<State<Vec<Message>>>,
    is_busy: Arc<Flag>,
) {
    let port = Settings::get().server.port;
    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{port}");

    let max_messages = Settings::get().assistant.max_messages * 2;
    let messages_limit = max_messages * 2;

    while let Some(input) = input_rx.recv().await {
        is_busy.set(true);
        let trimmed = input.trim();

        if trimmed == "/clear" {
            messages.set(vec![]).await;
            is_busy.set(false);
            continue;
        }

        // --- COMPRESS LOGIC ---
        let is_manual = trimmed == "/compact";
        let len = messages.get().await.len();
        if is_manual || len >= messages_limit {
            let _ = ui_tx.send(Chunk::Think {
                think: "⚙ Compacting context...".into(),
            });

            let compress_count = if is_manual {
                len.saturating_sub(2)
            } else {
                len.saturating_sub(max_messages)
            };

            if compress_count > 0 {
                let to_compress = messages.get().await.clone()[..compress_count].to_vec();

                let res = client
                    .post(format!("{base_url}/compact"))
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
                            messages.get().await.clone()[compress_count..].to_vec();

                        let mut messages_temp = vec![Message::assistant(vec![
                            format!("**Summarized**: {}\n---\n", summary.trim()).into(),
                        ])];
                        messages_temp.extend(preserved_msgs);
                        messages.set(messages_temp).await;
                    }
                }
            }
            if is_manual {
                is_busy.set(false);
                continue;
            }
        }

        let res = client
            .post(format!("{base_url}/handle"))
            .json(&UserQuery {
                user_id: 0,
                messages: (*messages.get().await).clone(),
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
                    error: format!("Connection error: {}", e),
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
    loop {
        app.tick_count += 1;
        terminal.draw(|f| ui(f, app)).map_err(|e| e.to_string())?;

        // read server chunks:
        while let Ok(chunk) = ui_rx.try_recv() {
            match chunk {
                Chunk::Think { .. } => app.is_thinking = true,
                Chunk::Answer { answer } => {
                    app.is_thinking = false;

                    let mut msgs = app.messages.lock().await;

                    match msgs.last_mut() {
                        Some(msg) if msg.role.is_assistant() => {
                            if let Some(Content::Text { text }) = msg.content.get_mut(0) {
                                text.push_str(&answer);
                            }
                        }
                        _ => {
                            msgs.push(Message::assistant(vec![answer.into()]));
                        }
                    }
                    app.scroll_offset = app.scroll_offset.saturating_add(1);
                }
                Chunk::Error { error } => {
                    app.is_thinking = false;
                    app.messages
                        .lock()
                        .await
                        .push(Message::system(vec![format!("Error: {error}").into()]));
                }
                Chunk::Spec { spec: _ } => {}
            }
        }

        // init keyboard events:
        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
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
                        KeyCode::Up => app.scroll_offset = app.scroll_offset.saturating_sub(1),
                        KeyCode::Down => app.scroll_offset = app.scroll_offset.saturating_add(1),
                        KeyCode::PageUp => app.scroll_offset = app.scroll_offset.saturating_sub(5),
                        KeyCode::PageDown => {
                            app.scroll_offset = app.scroll_offset.saturating_add(5)
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

/// Renderers the user interface
fn ui(f: &mut ratatui::Frame, app: &mut AppState) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // header
            Constraint::Min(1),    // chat
            Constraint::Length(3), // input
            Constraint::Length(1), // footer
        ])
        .split(area);

    render_header(f, chunks[0]);

    let chat_area = chunks[1];
    let max_width = chat_area.width.saturating_sub(2) as usize;

    let mut history: Vec<Line> = Vec::new();

    for msg in app.messages.dirty_get().iter() {
        // metadata line:
        let meta_line = Line::from(vec![if msg.role.is_user() {
            Span::styled(
                format!(
                    "[{}]",
                    msg.timestamp
                        .map(|t| {
                            t.with_timezone(&chrono::Local)
                                .format("%Y-%m-%dT%H:%M:%S")
                                .to_string()
                        })
                        .unwrap_or_else(|| "??:??:??".to_string())
                ),
                Style::default().bg(Color::Cyan).fg(Color::Black).bold(),
            )
        } else {
            Span::styled(str!(), Style::default())
        }]);
        history.push(meta_line);

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
        let msg_lines = parse_markdown(&text_content, max_width);

        if msg.role.is_user() {
            for line in msg_lines {
                let styled_line = line.clone();
                history.push(
                    styled_line.patch_style(
                        Style::default()
                            .bg(Color::Rgb(20, 20, 20))
                            .fg(Color::DarkGray),
                    ),
                );
            }
        } else {
            history.extend(msg_lines);
        }

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
    let dots_count = (app.tick_count / 30) % 4;
    let dots = ".".repeat(dots_count as usize);
    let thinking_text = format!(" Thinking{dots:<3} ");

    f.render_widget(
        Paragraph::new(format!("  → {}", app.input)).block(
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

    // --- CURSOR POSITIONING ---
    // x: the beginning of the block + prefix indentation (4) + cursor position in the line
    // y: the beginning of the block + 1 (offset from the upper border of the frame)
    if !app.is_thinking {
        let text_before_cursor = &app.input[..app.cursor_position];
        let visual_width = text_before_cursor.width();

        let x_offset = input_area.x + 1 + 4;

        f.set_cursor_position((x_offset + (visual_width as u16), input_area.y + 1));
    }

    // --- FOOTER ---
    render_footer(f, chunks[3], app);
}

/// Renders the footer section
fn render_footer(f: &mut ratatui::Frame, area: Rect, app: &AppState) {
    let commands = &app.commands;

    let index = (app.tick_count / 200) as usize % commands.len();
    let (cmd, desc) = commands[index];

    let help_line = Line::from(vec![
        " Tips: ".dim(),
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
        .title(format!(" Ovsy Assistant {} ", app_version()));

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
            Constraint::Length(2),      // left gap
            Constraint::Percentage(60), // left info block
            Constraint::Percentage(40), // right info block
            Constraint::Length(2),      // right gap
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
            Span::styled("Directory: ", Style::default().white().bold()),
            Span::styled(current_path, Style::default().gray()),
        ]),
    ]);

    // right info block:
    let right_text = Text::from(
        vec![Line::from(""), Line::from("")]
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
