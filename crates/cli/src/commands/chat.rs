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
use ovsy_share::{Chunk, ChunkData, CompactQuery, HandleQuery, SessionId, UserSessionsQuery};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    style::Stylize,
};
use std::{io, process::Command};
use tokio::{task::JoinHandle, time::Instant};

const FRAME_TIME: Duration = Duration::from_millis(33); // ~30 FPS
const USER_ID: u128 = 0;

/// Handles the CLI chat
pub async fn handle_chat() -> Result<()> {
    let port = Settings::get().server.port;

    // check server:
    let client = Client::tcp();
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

    // load last session or create new:
    let mut session_id = SessionId::new(USER_ID);
    let sessions_query = UserSessionsQuery::new(1); // limit: 1
    let base_url = str!("http://127.0.0.1:{port}");
    let sessions_url = str!("{base_url}/users/{USER_ID}/sessions");

    if let Ok(res) = client
        .post(&sessions_url)
        .json(&sessions_query)
        .send()
        .await
    {
        if let Ok(active_sessions) = res.json::<Vec<SessionId>>().await
            && let Some(last_session) = active_sessions.into_iter().next()
        {
            session_id = last_session;
        }
    }

    // init app state:
    let mut app = AppState::new(session_id, input_tx);

    // start chat worker:
    tokio::spawn(chat_worker(
        app.session_id.clone(),
        input_rx,
        ui_tx,
        app.messages.clone(),
    ));

    // render tui app:
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let res = run_app(&mut terminal, &mut app, &mut ui_rx).await;

    // --- ГАРАНТИРОВАННЫЙ ВЫХОД ИЗ RAW MODE ДО СЕТЕВЫХ ЗАПРОСОВ ---
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    // --- GRACEFUL SHUTDOWN: Закрытие сессии на бэкенде ---
    println!("Flushing DB records and closing session cleanly...");

    let finish_url = format!("http://127.0.0.1:{port}/sessions/{}/finish", app.session_id);
    let client = Client::tcp();

    if let Err(e) = client.post(&finish_url).send().await {
        eprintln!("Warning: Failed to finish session cleanly on backend: {e}");
    } else {
        println!("Ovsy session closed successfully.");
    }

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

        // collect all available chunks:
        let start = Instant::now();

        {
            let mut msgs = app.messages.lock().await;

            while start.elapsed() < FRAME_TIME {
                match ui_rx.try_recv() {
                    Ok(chunk) => {
                        handle_chunk(app, &mut msgs, chunk).await;
                    }
                    Err(_) => break,
                }
            }
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

/// Process input drivers and keyboard hooks
/// (returns true if application should terminate)
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
    session_id: Arc<State<SessionId>>,
    mut input_rx: mpsc::UnboundedReceiver<ChatAction>,
    ui_tx: mpsc::UnboundedSender<Chunk>,
    messages: Arc<State<Messages>>,
) {
    let port = Settings::get().server.port;
    let client = Client::tcp();
    let base_url = str!("http://127.0.0.1:{port}");

    // Замыкание для ленивой сборки SessionInfo из контекста CLI окружения
    let get_session_info = || {
        let tz_minutes = (chrono::Local::now().offset().local_minus_utc() / 60) as i16;
        ovsy_share::SessionInfo {
            current_path: std::env::current_dir().ok(),
            timezone: tz_minutes,
        }
    };

    // --- ПЕРВИЧНАЯ ИНИЦИАЛИЗАЦИЯ СЕССИИ ---
    let session_info = get_session_info();
    if let Ok(res) = client
        .post(&str!("{base_url}/sessions/{session_id}/init"))
        .json(&session_info)
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
                        "/new" => {
                            // Генерация нового ID локально
                            let new_sid = SessionId::new(USER_ID);
                            session_id.set(new_sid.clone()).await;
                            messages.set(Messages::new()).await;

                            // Регистрация и инициализация новой сессии на бэкенде
                            let session_info = get_session_info();
                            let _ = client
                                .post(&str!("{base_url}/sessions/{new_sid}/init"))
                                .json(&session_info)
                                .send()
                                .await;

                            let _ = ui_tx.send(Chunk {
                                data: ChunkData::Finish,
                                agent: None,
                            });
                        }

                        "/clear" | "/clean" => {
                            let ui_tx = ui_tx.clone();
                            let base_url = base_url.clone();
                            let messages = messages.clone();

                            let result = Client::tcp()
                                .post(&str!("{base_url}/sessions/{session_id}/clear"))
                                .send()
                                .await;

                            match result {
                                Ok(_) => {
                                    messages.set(Messages::new()).await;
                                }
                                Err(e) => {
                                    let _ = ui_tx.send(Chunk::error(str!(
                                        "Failed to clear remote history: {}",
                                        e
                                    )));
                                }
                            }

                            let _ = ui_tx.send(Chunk {
                                data: ChunkData::Finish,
                                agent: None,
                            });
                        }

                        "/compact" | "/compress" => {
                            let ui_tx = ui_tx.clone();
                            let base_url = base_url.clone();
                            let messages = messages.clone();
                            let session_id = session_id.clone();

                            current_task = Some(tokio::spawn(async move {
                                let _ = ui_tx.send(Chunk::think("Compressing context..."));

                                let preserve = args
                                    .get(1)
                                    .and_then(|i| i.trim().parse::<usize>().ok())
                                    .unwrap_or_else(|| Settings::get().assistant.preserve_messages);

                                let res = Client::tcp()
                                    .post(&str!("{base_url}/sessions/{session_id}/compact"))
                                    .json(&CompactQuery::new(preserve))
                                    .stream::<Chunk>()
                                    .await;

                                match res {
                                    Ok(mut stream) => {
                                        let mut msgs = messages.lock().await;

                                        let preserved_messages = msgs.slice(-(preserve as isize));

                                        msgs.messages = preserved_messages;
                                        msgs.count_tokens();

                                        while let Ok(Some(chunk)) = stream.recv().await {
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
                    let base_url = base_url.clone();
                    let ui_tx = ui_tx.clone();
                    let session_id = session_id.clone();

                    current_task = Some(tokio::spawn(async move {
                        let res = Client::tcp()
                            .post(&str!("{base_url}/sessions/{session_id}/query"))
                            .json(&HandleQuery::new(msg))
                            .stream::<Chunk>()
                            .await;

                        match res {
                            Ok(mut stream) => {
                                while let Ok(Some(chunk)) = stream.recv().await {
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
async fn handle_chunk(app: &mut AppState, msgs: &mut StateGuard<Messages>, chunk: Chunk) {
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
            let id_str = agent.as_ref().map(|task| task.tool_call_id.as_str());

            // push answer part into last message:
            msgs.push_str(id_str, &answer);
            app.chat_scroll = u16::MAX;
        }

        Chunk {
            data: ChunkData::Finish,
            agent,
        } => {
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
                let current_tool_id = task.tool_call_id.clone();

                // push error to last message:
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
        msgs.add_message(Message::user(vec!["This is a follow-up step. Review the results of all tool calls from the previous response. If the user's request has already been completed, provide the final answer. Otherwise, determine what remains to be done and continue by calling additional tools if necessary.".into()]));
        msgs.add_message(Message::assistant(vec![], vec![]));
    }

    let _ = tx.send(ChatAction::Query(str!()));
}
