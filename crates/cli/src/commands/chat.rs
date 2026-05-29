use crate::{
    chat::{AppState, markdown, utils},
    prelude::*,
};

use anylm::{Content, Message};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ovsy_shared::{Chunk, ChunkData, UserQuery};
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};
use reqwest::Client;
use std::{io, process::Command};

/// API: Handles the `chat` command
pub async fn handle() -> Result<()> {
    let port = Settings::get().server.port;

    // check server:
    let client = Client::new();
    let status_url = str!("http://127.0.0.1:{port}/update");

    if client.get(&status_url).send().await.is_err() {
        let bin_path = path!("$/ovsy-server{}", if cfg!(windows) { "exe" } else { "" });
        if Command::new(bin_path).arg("start").spawn().is_ok() {
            let mut is_ok = false;

            for _ in 0..10 {
                time::sleep(Duration::from_millis(500)).await;
                if client.get(&status_url).send().await.is_ok() {
                    is_ok = true;
                    break;
                }
            }

            if !is_ok {
                eprintln!(
                    "{}: Server started but is not responding.",
                    "Timeout".red().bold()
                );
            }
        } else {
            eprintln!("{}: Failed to execute server", "Error".red().bold());
        }
    }

    // init channels:
    let (input_tx, input_rx) = mpsc::unbounded_channel::<String>();
    let (ui_tx, mut ui_rx) = mpsc::unbounded_channel::<Chunk>();

    // init app state:
    let mut app = AppState::new(input_tx);

    // start chat worker:
    tokio::spawn(chat_worker(input_rx, ui_tx, app.messages.clone()));

    // render tui app:
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let res = run_app(&mut terminal, &mut app, &mut ui_rx).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    res
}

/// Runs the terminal app
async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut AppState,
    ui_rx: &mut mpsc::UnboundedReceiver<Chunk>,
) -> Result<()> {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(
            std::io::stdout(),
            LeaveAlternateScreen,
            event::DisableMouseCapture
        );
        default_hook(panic_info);
    }));

    execute!(std::io::stdout(), event::EnableMouseCapture)?;

    loop {
        app.tick_count += 1;
        terminal
            .draw(|f| render_tui(f, app))
            .map_err(|e| e.to_string())?;

        // 1. Processing incoming data from the server stream
        if let Ok(chunk) = ui_rx.try_recv() {
            handle_chunk(app, chunk).await;
        }

        // 2. Event Handling
        if event::poll(std::time::Duration::from_millis(16))? {
            if handle_event(app, event::read()?).await? {
                break;
            }
        }
    }
    Ok(())
}

/// Process input drivers and keyboard hooks. Returns true if application should terminate.
async fn handle_event(app: &mut AppState, event: Event) -> Result<bool> {
    match event {
        Event::Mouse(mouse_event) => {
            let pos = (mouse_event.column, mouse_event.row).into();
            match mouse_event.kind {
                event::MouseEventKind::ScrollUp => {
                    if app.chat_area.contains(pos) {
                        app.chat_scroll = app.chat_scroll.saturating_sub(2);
                    } else if app.input_area.contains(pos) {
                        app.input_scroll = app.input_scroll.saturating_sub(1);
                    }
                }
                event::MouseEventKind::ScrollDown => {
                    if app.chat_area.contains(pos) {
                        app.chat_scroll = app.chat_scroll.saturating_add(2);
                    } else if app.input_area.contains(pos) {
                        app.input_scroll = app.input_scroll.saturating_add(1);
                    }
                }
                _ => {}
            }
        }
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            let has_shift = key.modifiers.contains(event::KeyModifiers::SHIFT);

            match key.code {
                KeyCode::Esc => {
                    let _ = execute!(std::io::stdout(), event::DisableMouseCapture);
                    return Ok(true);
                }
                KeyCode::Enter => {
                    if app.input.trim() == "/exit" {
                        let _ = execute!(std::io::stdout(), event::DisableMouseCapture);
                        return Ok(true);
                    }
                    if !app.is_busy {
                        handle_input(app).await;
                    }
                }
                KeyCode::Char(c) => {
                    app.input.insert(app.input_cursor, c);
                    app.input_cursor += c.len_utf8();
                }
                KeyCode::Backspace if app.input_cursor > 0 => {
                    if let Some((i, _)) = app
                        .input
                        .char_indices()
                        .filter(|&(i, _)| i < app.input_cursor)
                        .last()
                    {
                        app.input.remove(i);
                        app.input_cursor = i;
                    }
                }
                KeyCode::Left if app.input_cursor > 0 => {
                    app.input_cursor = app
                        .input
                        .char_indices()
                        .filter(|&(i, _)| i < app.input_cursor)
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                }
                KeyCode::Right => {
                    if let Some((i, c)) = app
                        .input
                        .char_indices()
                        .find(|&(i, _)| i == app.input_cursor)
                    {
                        app.input_cursor = i + c.len_utf8();
                    }
                }
                KeyCode::Up => app.chat_scroll = app.chat_scroll.saturating_sub(1),
                KeyCode::Down => app.chat_scroll = app.chat_scroll.saturating_add(1),
                KeyCode::PageUp => {
                    if has_shift {
                        app.input_scroll = app.input_scroll.saturating_sub(5);
                    } else {
                        app.chat_scroll = app.chat_scroll.saturating_sub(10);
                    }
                }
                KeyCode::PageDown => {
                    if has_shift {
                        app.input_scroll = app.input_scroll.saturating_add(5);
                    } else {
                        app.chat_scroll = app.chat_scroll.saturating_add(10);
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(false)
}

/// Handles the user input
async fn handle_input(app: &mut AppState) {
    app.is_busy = true;

    let trimmed = app.input.trim();
    if trimmed.is_empty() {
        return;
    }

    // handle input:
    if trimmed.starts_with('/') {
        match trimmed {
            "/exit" => {}
            _ => {
                let _ = app.tx.send(trimmed.to_string());
            }
        }
    } else {
        // add user message:
        {
            let mut msgs = app.messages.lock().await;
            msgs.push(Message::user(vec![trimmed.into()]));
            msgs.push(Message::assistant(vec![], vec![]));
            app.response_index = msgs.len() - 1;
        }

        // send query to worker:
        let _ = app.tx.send(trimmed.to_string());
    }

    app.input.clear();
    app.input_cursor = 0;
    app.input_scroll = 0;
    app.chat_scroll = u16::MAX;
}

/// A worker for networking
async fn chat_worker(
    mut input_rx: mpsc::UnboundedReceiver<String>,
    ui_tx: mpsc::UnboundedSender<Chunk>,
    messages: Arc<Mutex<Vec<Message>>>,
) {
    let port = Settings::get().server.port;
    let client = reqwest::Client::new();
    let base_url = str!("http://127.0.0.1:{port}");

    while let Some(input) = input_rx.recv().await {
        let trimmed = input.trim();

        if trimmed.starts_with('/') {
            let args: Vec<String> = trimmed.split_whitespace().map(|s| s.to_string()).collect();
            if args.is_empty() {
                continue;
            }

            match args[0].to_lowercase().as_str() {
                "/clear" | "/clean" => {
                    messages.lock().await.clear();
                }
                "/compact" | "/compress" => {
                    let _ = ui_tx.send(Chunk::think("Compressing context..."));
                    let preserve = args
                        .get(1)
                        .and_then(|i| i.trim().parse::<usize>().ok())
                        .unwrap_or_else(|| Settings::get().assistant.preserve_messages);

                    let msgs = messages.lock().await;
                    let msgs_len = msgs.len();

                    if msgs_len > preserve {
                        let chunks = utils::split_messages(msgs.clone());
                        if chunks.len() > preserve {
                            let to_compress: Vec<Message> = chunks[..chunks.len() - preserve]
                                .iter()
                                .flatten()
                                .cloned()
                                .collect();
                            drop(msgs);

                            let res = client
                                .post(str!("{base_url}/compact"))
                                .json(&UserQuery {
                                    user_id: 0,
                                    messages: to_compress,
                                })
                                .send()
                                .await;

                            if let Ok(res) = res {
                                let mut stream = Stream::read::<Chunk>(
                                    res.bytes_stream().map(|c| c.map_err(Into::into)),
                                );
                                let mut new_messages = vec![Message::assistant(
                                    vec![str!("# Summarized: \n").into()],
                                    vec![],
                                )];

                                let mut msgs = messages.lock().await;
                                new_messages.extend(msgs[msgs_len - preserve..].to_vec());
                                *msgs = new_messages;

                                while let Ok(Some(Chunk {
                                    data: ChunkData::Answer(part),
                                    ..
                                })) = stream.read().await
                                {
                                    if let Some(msg) = msgs.last_mut() {
                                        msg.map(|cnt| {
                                            if let Some(Content::Text { text }) = cnt.get_mut(0) {
                                                text.push_str(&part);
                                            }
                                        });
                                    }
                                }

                                if let Some(msg) = msgs.last_mut() {
                                    msg.map(|cnt| {
                                        if let Some(Content::Text { text }) = cnt.get_mut(0) {
                                            text.push_str("\n---");
                                        }
                                    });
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
            // Signal TUI that command execution finished
            let _ = ui_tx.send(Chunk {
                data: ChunkData::Finish,
                agent: None,
            });
            continue;
        }

        let mut req_messages = messages.lock().await.clone();
        if !req_messages.is_empty() {
            req_messages.pop();
        }

        let res = client
            .post(str!("{base_url}/handle"))
            .json(&UserQuery {
                user_id: 0,
                messages: req_messages,
            })
            .send()
            .await;

        match res {
            Ok(response) => {
                let mut stream =
                    Stream::read::<Chunk>(response.bytes_stream().map(|c| c.map_err(Into::into)));
                while let Ok(Some(chunk)) = stream.read().await {
                    let _ = ui_tx.send(chunk);
                }
            }
            Err(e) => {
                let _ = ui_tx.send(Chunk::error(str!("Connection error: {}", e)));
            }
        }
    }
}

/// Process backend runtime text chunks
async fn handle_chunk(app: &mut AppState, chunk: Chunk) {
    match chunk {
        Chunk {
            data: ChunkData::Thinking(think),
            ..
        } => {
            app.status.replace(think);
        }
        Chunk {
            data: ChunkData::Tools(tool_calls),
            ..
        } => {
            if let Some(msg) = app.messages.lock().await.get_mut(app.response_index) {
                msg.tool_calls.extend(tool_calls);
            }
        }

        Chunk {
            agent,
            data: ChunkData::Answer(answer),
        } => {
            let mut msgs = app.messages.lock().await;

            if let Some(task) = agent {
                let current_tool_id = task.tool_call_id;
                match msgs.last_mut() {
                    Some(msg) if msg.role.is_tool() && msg.tool_call_id == current_tool_id => {
                        msg.map(|cnt| {
                            if let Some(Content::Text { text }) = cnt.get_mut(0) {
                                text.push_str(&answer);
                            }
                        });
                    }
                    _ => msgs.push(Message::tool(vec![answer.into()], current_tool_id)),
                }
            } else if let Some(msg) = msgs.get_mut(app.response_index) {
                msg.map(|cnt| {
                    if cnt.is_empty() {
                        cnt.push(answer.clone().into());
                    } else if let Some(Content::Text { text }) = cnt.get_mut(0) {
                        text.push_str(&answer);
                    }
                });
            }
            app.chat_scroll = u16::MAX;
        }

        Chunk {
            data: ChunkData::Finish,
            agent,
        } => {
            let mut msgs = app.messages.lock().await;
            let idx = app.response_index;

            if idx < msgs.len() && !msgs[idx].tool_calls.is_empty() {
                let ordered_ids: Vec<String> = msgs[idx]
                    .tool_calls
                    .iter()
                    .map(|tc| tc.id.clone())
                    .collect();
                let remaining = msgs.drain((idx + 1)..).collect::<Vec<_>>();

                let (mut tool_messages, other_messages): (Vec<_>, Vec<_>) =
                    remaining.into_iter().partition(|m| m.role.is_tool());

                tool_messages.sort_by_key(|msg| {
                    ordered_ids
                        .iter()
                        .position(|id| id == &msg.tool_call_id)
                        .unwrap_or(usize::MAX)
                });

                msgs.extend(tool_messages);
                msgs.extend(other_messages);
            }

            if agent.is_none() {
                app.status.take();

                // do control query:
                if let Some(last_msg) = msgs.last()
                    && last_msg.role.is_tool()
                {
                    let tx = app.tx.clone();
                    let messages = app.messages.clone();
                    app.response_index = msgs.len() + 1;

                    tokio::spawn(async move {
                        handle_control_query(tx, messages).await;
                    });
                } else {
                    app.is_busy = false;
                }
            }
        }

        Chunk {
            data: ChunkData::Error(error),
            ..
        } => {
            app.is_busy = false;
            app.messages
                .lock()
                .await
                .push(Message::system(vec![str!("Error: {error}").into()]));
        }
    }
}

/// Handles the control query
async fn handle_control_query(
    tx: mpsc::UnboundedSender<String>,
    messages: Arc<Mutex<Vec<Message>>>,
) {
    {
        let mut msgs = messages.lock().await;
        msgs.push(Message::user(vec!["".into()]));
        msgs.push(Message::assistant(vec![], vec![]));
    }

    let _ = tx.send(String::new());
}

// --------------------- TUI RENDER's ----------------------------

/// Renderers the user interface
fn render_tui(f: &mut Frame, app: &mut AppState) {
    let area = f.area();

    let Ok(messages_guard) = app.messages.try_lock() else {
        return;
    };

    // --- 1. CALCULATE INPUT HEIGHT ---
    let prefix = " ❯ ";
    let inner_width = area.width.saturating_sub(2) as usize;

    let wrap_options = textwrap::Options::new(inner_width)
        .break_words(true)
        .word_separator(textwrap::WordSeparator::AsciiSpace);

    let full_text = str!("{prefix}{}", app.input);
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

    // render messages:
    for msg in messages_guard.iter() {
        // collect content:
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

        // metadata line:
        if msg.role.is_user() {
            let meta_line = Line::from(vec![Span::styled(
                str!(
                    "[{}]",
                    msg.timestamp
                        .map(|t| t
                            .with_timezone(&Local)
                            .format("%Y-%m-%d|%H:%M:%S")
                            .to_string())
                        .unwrap_or_else(|| "??:??:??".to_string())
                ),
                Style::default().bg(Color::Cyan).fg(Color::Black).bold(),
            )]);
            history.push(meta_line);
        } else if !history.is_empty() {
            history.push(Line::raw(""));
        }

        let msg_lines = markdown::parse(&text_content, chat_inner_width as usize);

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
        } else if msg.role.is_tool() {
            for line in msg_lines {
                history.push(line.patch_style(Style::default().bg(Color::Rgb(25, 25, 25))));
            }
        } else {
            history.extend(msg_lines);
        }
    }

    let total_lines = history.len() as u16;
    let chat_height = chat_area.height.saturating_sub(2) as u16;
    let max_chat_scroll = total_lines.saturating_sub(chat_inner_height);
    app.chat_scroll = app.chat_scroll.min(max_chat_scroll);

    // calculate scroll lines:
    let current_line = if total_lines <= chat_height {
        total_lines
    } else {
        (app.chat_scroll + chat_height).min(total_lines)
    };

    let max_tokens = Settings::get()
        .assistant
        .completions
        .max_tokens
        .unwrap_or(1) as usize;

    let tokens_count = utils::count_tokens(&messages_guard);

    fn make_progress_bar(current: usize, max: usize, width: usize) -> String {
        if max == 0 {
            return " ".repeat(width);
        }
        let ratio = (current as f32 / max as f32).clamp(0.0, 1.0);
        let filled_len = (ratio * width as f32).round() as usize;
        let empty_len = width.saturating_sub(filled_len);

        str!("{}{}", "■".repeat(filled_len), "□".repeat(empty_len))
    }

    let pb = make_progress_bar(tokens_count, max_tokens, 10);
    let tokens_text = str!("{}/{}", tokens_count, max_tokens);
    let lines_text = str!(
        "{}/{}",
        current_line.saturating_sub(1),
        total_lines.saturating_sub(1)
    );

    let ratio = if max_tokens > 0 {
        (tokens_count as f32 / max_tokens as f32).clamp(0.0, 1.0)
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
        Span::styled(lines_text, Style::default().fg(Color::White)),
        Span::raw(" "),
    ]);

    let title_right2 = Line::from(vec![Span::raw(" ")]);

    f.render_widget(
        Paragraph::new(history).scroll((app.chat_scroll, 0)).block(
            Block::default()
                .borders(Borders::LEFT | Borders::TOP)
                .border_style(Style::default().dim())
                .title(title_left)
                .title(title_right.alignment(ratatui::layout::HorizontalAlignment::Right))
                .title(title_right2.alignment(ratatui::layout::HorizontalAlignment::Right)),
        ),
        chat_area,
    );

    // free lock:
    drop(messages_guard);

    // --- 5. INPUT BLOCK ---
    let input_area = chunks[2];
    let input_inner_height = input_area.height.saturating_sub(2);

    // prepare input text:
    let text_before_cursor = &app.input[..app.input_cursor];
    let text_temp = str!("{}{}_", prefix, text_before_cursor);
    let wrapped_before = textwrap::wrap(&text_temp, wrap_options.clone());
    let cursor_row = (wrapped_before.len() as u16).saturating_sub(1);

    // input scroll:
    if cursor_row >= app.input_scroll + input_inner_height {
        app.input_scroll = cursor_row - input_inner_height + 1;
    }
    if cursor_row < app.input_scroll {
        app.input_scroll = cursor_row;
    }

    let dots_count = (app.tick_count / 30) % 4;
    let dots = ".".repeat(dots_count as usize);
    let thinking_text = str!(" Thinking{dots:<3} ");

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                prefix,
                if app.is_busy {
                    Color::White
                } else {
                    Color::Cyan
                },
            ),
            app.input.as_str().into(),
        ]))
        .wrap(Wrap { trim: false })
        .scroll((app.input_scroll, 0))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(if app.is_busy {
                    Span::styled(thinking_text, Style::default().fg(Color::White).italic())
                } else {
                    Span::styled(" Input ", Style::default().fg(Color::Cyan))
                })
                .border_style(Style::default().fg(if app.is_busy {
                    Color::White
                } else {
                    Color::Cyan
                })),
        ),
        input_area,
    );

    // --- 6. CURSOR POSITION ---
    let last_line_len = wrapped_before
        .last()
        .map(|s| s.chars().count())
        .unwrap_or(0);
    let last_line = last_line_len.saturating_sub(1);

    f.set_cursor_position((
        input_area.x + 1 + last_line as u16,
        input_area.y + 1 + cursor_row - app.input_scroll,
    ));

    // --- 7. FOOTER ---
    render_footer(f, chunks[3], app);
}

/// Renders the header section
fn render_header(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(str!(" Ovsy {} ", app_version()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3)])
        .split(inner);

    let content_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(2),
            Constraint::Percentage(100),
            Constraint::Length(2),
        ])
        .split(layout[0]);

    let current_path = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "Unknown".to_string());

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

/// Renders the footer section
fn render_footer(f: &mut Frame, area: Rect, app: &AppState) {
    let help_line = if let Some(status) = app.status.as_ref().filter(|s| !s.is_empty()) {
        let parsed_lines = markdown::parse(status, usize::MAX);

        let mut spans = vec!["  ".into()];
        for line in parsed_lines {
            for span in line.spans {
                spans.push(span.italic());
            }
        }

        if spans.len() > 1 {
            Line::from(spans)
        } else {
            Line::from(vec!["  ".into(), status.as_str().dim().italic()])
        }
    } else {
        let commands = &app.commands;
        let index = (app.tick_count / 360) as usize % commands.len();
        let (cmd, desc) = commands[index];

        Line::from(vec![
            "  ".into(),
            cmd.bold().cyan(),
            " ".into(),
            desc.gray().italic(),
        ])
    };

    f.render_widget(Paragraph::new(help_line).alignment(Alignment::Left), area);
}
