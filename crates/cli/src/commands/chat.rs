use crate::{
    chat::{self, AppState, ChatAction},
    prelude::*,
};

use anylm::{Message, Messages};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ovsy_share::{Chunk, ChunkData, CompactQuery, HandleQuery, SessionID, UserSessionsQuery};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    style::Stylize,
};
use reqwest::Client;
use std::{io, process::Command};
use tokio::task::JoinHandle;

const USER_ID: u128 = 0;

/// API: Handles the `chat` command
pub async fn handle_chat() -> Result<()> {
    let port = Settings::get().server.port;

    // check server:
    let client = Client::new();
    let status_url = str!("http://127.0.0.1:{port}/update");

    if client.get(&status_url).send().await.is_err() {
        let bin_path = path!("$/ovsy-core{}", if cfg!(windows) { "exe" } else { "" });
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
    let (input_tx, input_rx) = mpsc::unbounded_channel::<ChatAction>();
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
            .draw(|f| chat::render_tui(f, app))
            .map_err(|e| e.to_string())?;

        // processing incoming data from the server:
        if let Ok(chunk) = ui_rx.try_recv() {
            handle_chunk(app, chunk).await;
        }

        // handling terminal event:
        if event::poll(Duration::from_millis(16))? {
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
                    match app.input.trim() {
                        "/exit" | "/quit" => {
                            let _ = execute!(std::io::stdout(), event::DisableMouseCapture);
                            return Ok(true);
                        }

                        "/cancel" | "/stop" => {
                            app.input.clear();
                            app.input_cursor = 0;
                            app.is_canceled = true;
                            let _ = app.tx.send(ChatAction::Cancel);
                            return Ok(false);
                        }

                        _ => {}
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
    app.is_canceled = false;
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
                let _ = app.tx.send(ChatAction::Query(trimmed.into()));
            }
        }
    } else {
        // reset cycles:
        app.cycles = 0;

        // add user message:
        {
            let mut msgs = app.messages.lock().await;
            msgs.add_message(Message::user(vec![trimmed.into()]));
            msgs.add_message(Message::assistant(vec![], vec![]));
            app.response_index = msgs.messages.len() - 1;
        }

        // send query to worker:
        let _ = app.tx.send(ChatAction::Query(trimmed.into()));
    }

    app.input.clear();
    app.input_cursor = 0;
    app.input_scroll = 0;
    app.chat_scroll = u16::MAX;
}

/// A worker for networking
async fn chat_worker(
    mut input_rx: mpsc::UnboundedReceiver<ChatAction>,
    ui_tx: mpsc::UnboundedSender<Chunk>,
    messages: Arc<State<Messages>>,
) {
    let port = Settings::get().server.port;
    let client = reqwest::Client::new();
    let base_url = str!("http://127.0.0.1:{port}");

    let mut session_id = SessionID::new(USER_ID);

    // load last session or create new:
    let sessions_query = UserSessionsQuery::new(1); // limit: 1
    let sessions_url = str!("{base_url}/users/{USER_ID}/sessions");

    if let Ok(res) = client
        .post(&sessions_url)
        .json(&sessions_query)
        .send()
        .await
    {
        if let Ok(active_sessions) = res.json::<Vec<SessionID>>().await
            && let Some(last_session) = active_sessions.into_iter().next()
        {
            session_id = last_session;
        }
    }

    // load session history:
    if let Ok(res) = client
        .post(str!("{base_url}/sessions/{session_id}/get"))
        .send()
        .await
    {
        if let Ok(history) = res.json::<Vec<Message>>().await {
            let mut msgs = messages.lock().await;
            msgs.messages = history;
            msgs.count_tokens();
            msgs.sync();
        }
    }

    let mut current_task: Option<JoinHandle<()>> = None;

    // read chat actions:
    while let Some(action) = input_rx.recv().await {
        match action {
            ChatAction::Cancel => {
                if let Some(task) = current_task.take() {
                    task.abort();
                    let _ = ui_tx.send(Chunk {
                        data: ChunkData::Finish,
                        agent: None,
                    });
                }
                continue;
            }

            ChatAction::Query(input) => {
                if let Some(task) = current_task.take() {
                    task.abort();
                }

                let trimmed = input.trim();

                if trimmed.starts_with('/') {
                    let args: Vec<String> =
                        trimmed.split_whitespace().map(|s| s.to_string()).collect();
                    if args.is_empty() {
                        continue;
                    }

                    match args[0].to_lowercase().as_str() {
                        "/clear" | "/clean" => {
                            let ui_tx = ui_tx.clone();
                            let client = client.clone();
                            let base_url = base_url.clone();
                            let messages = messages.clone();
                            let session_id = session_id.clone();

                            let res = client
                                .post(str!("{base_url}/sessions/{session_id}/clear"))
                                .send()
                                .await;

                            {
                                let mut msgs = messages.lock().await;
                                msgs.messages.clear();
                                msgs.tokens_count = 0;
                                msgs.sync();
                            }

                            if let Err(e) = res {
                                let _ = ui_tx.send(Chunk::error(str!(
                                    "Failed to clear remote history: {}",
                                    e
                                )));
                            }

                            let _ = ui_tx.send(Chunk {
                                data: ChunkData::Finish,
                                agent: None,
                            });
                        }

                        "/compact" | "/compress" => {
                            let ui_tx = ui_tx.clone();
                            let client = client.clone();
                            let base_url = base_url.clone();
                            let messages = messages.clone();
                            let session_id = session_id.clone();

                            current_task = Some(tokio::spawn(async move {
                                let _ = ui_tx.send(Chunk::think("Compressing context..."));

                                let preserve = args
                                    .get(1)
                                    .and_then(|i| i.trim().parse::<usize>().ok())
                                    .unwrap_or_else(|| Settings::get().assistant.preserve_messages);

                                let res = client
                                    .post(str!("{base_url}/sessions/{session_id}/compact"))
                                    .json(&CompactQuery::new(preserve))
                                    .send()
                                    .await;

                                match res {
                                    Ok(response) => {
                                        let mut msgs = messages.lock().await;

                                        let preserved_messages = msgs.slice(-(preserve as isize));

                                        msgs.messages = preserved_messages;
                                        msgs.count_tokens();

                                        let bytes_stream =
                                            response.bytes_stream().map(|c| c.map_err(Into::into));
                                        let mut stream = Stream::read::<Chunk>(bytes_stream);

                                        while let Ok(Some(chunk)) = stream.read().await {
                                            if let ChunkData::Answer(ref text) = chunk.data {
                                                msgs.push_str(None, text);
                                            }
                                            let _ = ui_tx.send(chunk);
                                            msgs.sync_n(2);
                                        }

                                        let mut new_history =
                                            Vec::with_capacity(msgs.messages.len() + 1);
                                        new_history.extend(msgs.messages.clone());

                                        msgs.messages = new_history;
                                        msgs.count_tokens();
                                        drop(msgs);
                                    }
                                    Err(e) => {
                                        let _ = ui_tx
                                            .send(Chunk::error(str!("Compression failed: {}", e)));
                                    }
                                }

                                let _ = ui_tx.send(Chunk {
                                    data: ChunkData::Finish,
                                    agent: None,
                                });
                            }));
                        }
                        _ => {}
                    }

                    let _ = ui_tx.send(Chunk {
                        data: ChunkData::Finish,
                        agent: None,
                    });
                    continue;
                }

                let message_to_send = {
                    let msgs = messages.lock().await;
                    msgs.messages
                        .get(msgs.messages.len().saturating_sub(2))
                        .cloned()
                };

                if let Some(msg) = message_to_send {
                    let client = client.clone();
                    let base_url = base_url.clone();
                    let ui_tx = ui_tx.clone();
                    let session_id = session_id.clone();

                    current_task = Some(tokio::spawn(async move {
                        let res = client
                            .post(str!("{base_url}/sessions/{session_id}/query"))
                            .json(&HandleQuery::new(msg))
                            .send()
                            .await;

                        match res {
                            Ok(response) => {
                                let mut stream = Stream::read::<Chunk>(
                                    response.bytes_stream().map(|c| c.map_err(Into::into)),
                                );
                                while let Ok(Some(chunk)) = stream.read().await {
                                    let _ = ui_tx.send(chunk);
                                }
                            }
                            Err(e) => {
                                let _ = ui_tx.send(Chunk::error(str!("Connection error: {}", e)));
                            }
                        }
                    }));
                }
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
            let mut msgs = app.messages.lock().await;
            if let Some(msg) = msgs.messages.get_mut(app.response_index) {
                msg.tool_calls.extend(tool_calls);
                msg.count_tokens();
                msgs.count_tokens();
            }
        }

        Chunk {
            agent,
            data: ChunkData::Answer(answer),
        } => {
            let mut msgs = app.messages.lock().await;
            let id_str = agent.as_ref().map(|task| task.tool_call_id.as_str());

            // push_str атомарно находит нужный индекс или создает сообщение (tool/assistant)
            msgs.push_str(id_str, &answer);
            app.chat_scroll = u16::MAX;
        }

        Chunk {
            data: ChunkData::Finish,
            agent,
        } => {
            let mut msgs = app.messages.lock().await;
            let idx = app.response_index;

            if idx < msgs.messages.len() && !msgs.messages[idx].tool_calls.is_empty() {
                let ordered_ids: Vec<String> = msgs.messages[idx]
                    .tool_calls
                    .iter()
                    .map(|tc| tc.id.clone())
                    .collect();

                let remaining = msgs.messages.drain((idx + 1)..).collect::<Vec<_>>();

                let (mut tool_messages, other_messages): (Vec<_>, Vec<_>) =
                    remaining.into_iter().partition(|m| m.role.is_tool());

                tool_messages.sort_by_key(|msg| {
                    ordered_ids
                        .iter()
                        .position(|id| id == &msg.tool_call_id)
                        .unwrap_or(usize::MAX)
                });

                msgs.messages.extend(tool_messages);
                msgs.messages.extend(other_messages);
                msgs.count_tokens();
            }

            if agent.is_none() {
                app.status.take();

                // do control query:
                if app.cycles <= Settings::get().assistant.max_cycles {
                    if let Some(last_msg) = msgs.messages.last()
                        && last_msg.role.is_tool()
                    {
                        let tx = app.tx.clone();
                        let messages = app.messages.clone();
                        app.response_index = msgs.messages.len() + 1;
                        app.cycles += 1;

                        tokio::spawn(async move {
                            handle_control_query(tx, messages).await;
                        });
                    } else {
                        app.is_busy = false;
                    }
                }
            }
        }

        Chunk {
            data: ChunkData::Error(error),
            agent,
        } => {
            let err_msg = str!("Error: {error}");

            if let Some(task) = agent {
                let mut msgs = app.messages.lock().await;
                let current_tool_id = task.tool_call_id.clone();

                // Используем push_str для безопасного обновления контента сообщения ошибки инструмента
                msgs.push_str(Some(&current_tool_id), &format!("\n{}", err_msg));
                app.chat_scroll = u16::MAX;

                // trying to fix error:
                if app.cycles <= Settings::get().assistant.max_cycles {
                    if !app.is_canceled {
                        let tx = app.tx.clone();
                        let messages = app.messages.clone();
                        app.response_index = msgs.messages.len() + 1;
                        app.cycles += 1;

                        tokio::spawn(async move {
                            handle_control_query(tx, messages).await;
                        });
                    }
                }
            } else {
                app.is_busy = false;
                app.status.take();
                let mut msgs = app.messages.lock().await;
                msgs.add_message(Message::system(vec![
                    str!("Critical Error: {error}").into(),
                ]));
            }
        }
    }
}

/// Handles the control query
async fn handle_control_query(
    tx: mpsc::UnboundedSender<ChatAction>,
    messages: Arc<State<Messages>>,
) {
    {
        let mut msgs = messages.lock().await;
        msgs.add_message(Message::user(vec!["".into()]));
        msgs.add_message(Message::assistant(vec![], vec![]));
    }

    let _ = tx.send(ChatAction::Query(str!()));
}
