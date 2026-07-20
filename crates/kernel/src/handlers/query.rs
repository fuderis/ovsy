use crate::{Runtime, Session, manager::*, prelude::*, settings::AssistantOptions};

use anylm::{AiChunk, Completions, Content, Message, Messages};
use chrono::FixedOffset;
use ovsy_share::{Event, EventKind, HandleQuery, SessionInfo};
use std::collections::HashSet;
use tokio::task::JoinSet;

/// API: The user query handler
pub async fn handle_user_query(Paths(sid): Paths<SessionId>, data: Json<HandleQuery>) -> Response {
    let HandleQuery { message } = data.0;

    Response::ok().stream(move |tx| async move {
        let result = match read_session(sid).await {
            Ok((session, messages)) => {
                handle_query(sid, tx.clone(), session, messages, message).await
            }
            Err(e) => Err(e),
        };

        if let Err(e) = result {
            error!("[handle_query{{sid={sid}}}] {e}");
            tx.send(Event::error(str!(e))).ok();
        }
    })
}

/// Helper method to read user session from database
#[log(skip_all, fields(sid = %sid))]
async fn read_session(sid: SessionId) -> Result<(Arc<Mutex<Session>>, Arc<Mutex<Messages>>)> {
    info!("Reading the user session...");

    // init session & read messages:
    let Some(session) = Session::get(&sid) else {
        return Err(Error::UnknownSessionId(sid).into());
    };
    let db_messages = session.lock().await.read_messages().await?;
    let messages = arc_mutex!(Messages::from(db_messages));

    Ok((session, messages))
}

/// Handles the user query with self-healing on planning/generation level
#[log(skip_all, fields(sid = %sid))]
async fn handle_query(
    sid: SessionId,
    tx: Sender<Bytes>,
    session: Arc<Mutex<Session>>,
    messages: Arc<Mutex<Messages>>,
    message: Message,
) -> Result<()> {
    info!("Processing the user query...");

    let ai_conf = &Settings::get().assistant;
    let options = ai_conf.completions.clone();

    // prepare messages:
    let raw_messages = messages.lock().await.messages.clone();
    let messages = Messages::from(raw_messages)
        .system(vec![
            system_prompt(&session.lock().await.info, &ai_conf).into(),
            ai_conf
                .assist_prompt
                .replace("{AGENTS_LIST}", &Manager::agents_list_doc().await)
                .into(),
        ])
        .message(message)
        .wrap();

    let mut tasks_list = vec![];
    let mut evals_list = vec![];
    let mut retry_count = 0;
    let max_retries = ai_conf.max_retries.max(1) as usize;

    #[derive(Deserialize)]
    struct EvalAction {
        task_id: Option<i64>,
        parameter: Option<String>,
        code: String,
    }

    // top-level generation cycle: task planning
    loop {
        tasks_list.clear();
        evals_list.clear();
        let mut text_response = str!();

        let mut response = match Completions::try_from(options.clone())?
            .tools(Manager::basic_tools().await)
            .send(messages.clone())
            .await
        {
            Ok(res) => res,
            Err(e) => {
                retry_count += 1;
                if retry_count < max_retries {
                    warn!(
                        "Failed to send query completions request (attempt {retry_count}/{max_retries}): {e}"
                    );
                    messages.lock().await.add_user(vec![
                        format!("An error occurred: {e}. Please try again to plan the task using the tools.").into()
                    ]);
                    continue;
                } else {
                    return Err(e.into());
                }
            }
        };

        // read ai chunks and collect tool calls
        let mut chunk_error = None;
        while let Some(chunk) = response.next().await {
            match chunk {
                Ok(AiChunk::Text(text_part)) => {
                    text_response.push_str(&text_part);
                    // streaming plain text to the user
                    tx.send(Event::answer(text_part))?;
                }

                Ok(AiChunk::Tool(tool_call)) => match tool_call.func.name.as_ref() {
                    "handle_agent" => match tool_call.parse_args::<TaskAction>() {
                        Ok(mut task) => {
                            task.tool_call_id = tool_call.id;
                            tasks_list.push(task);
                        }
                        Err(e) => {
                            chunk_error = Some(str!("Failed to parse handle_agent: {e}").into());
                            break;
                        }
                    },
                    "javascript_eval" => match tool_call.parse_args::<EvalAction>() {
                        Ok(eval) => {
                            evals_list.push((tool_call.id, eval));
                        }
                        Err(e) => {
                            chunk_error = Some(str!("Failed to parse javascript_eval: {e}").into());
                            break;
                        }
                    },
                    _ => {}
                },

                Err(e) => {
                    chunk_error = Some(e.into());
                    break;
                }
            }
        }

        if let Some(err) = chunk_error {
            retry_count += 1;
            if retry_count < max_retries {
                warn!("Stream error on planning level ({retry_count}/{max_retries}): {err}");
                messages.lock().await.add_user(vec![
                    format!("An error occurred during stream generation: {err}. Please try again to complete the request.").into()
                ]);
                continue;
            } else {
                return Err(err);
            }
        }

        // hallucination check (if there is no text, no tasks, no JS calculations)
        if tasks_list.is_empty() && evals_list.is_empty() && text_response.trim().is_empty() {
            retry_count += 1;
            if retry_count < max_retries {
                warn!(
                    "Model hallucinated: empty text response and no tool calls. Retrying ({retry_count}/{max_retries})..."
                );
                messages.lock().await.add_user(vec![
                    "You returned an empty response. If you need to solve the task, delegate work to an agent (using handle_agent) or execute JS (using javascript_eval).".into()
                ]);
                continue;
            } else {
                return Err(str!("Model failed to plan tasks: returned empty response").into());
            }
        }

        break;
    }

    // performing JS calculations (if any)
    if !evals_list.is_empty() {
        let mut runtime = Runtime::new();

        for (tool_call_id, eval) in evals_list {
            let result: String = runtime.eval(&eval.code)?;

            if let Some(task_id) = eval.task_id {
                let Some(task) = tasks_list.iter_mut().find(|t| t.task_id == task_id) else {
                    warn!("Task #{task_id} not found");
                    continue;
                };

                if let Some(parameter) = &eval.parameter {
                    let placeholder = format!("{{{{{parameter}}}}}");

                    if task.task_query.contains(&placeholder) {
                        task.task_query = task.task_query.replace(&placeholder, &result);
                    } else {
                        warn!("Placeholder '{parameter}' not found in task #{task_id}");
                    }
                } else {
                    if !task.task_query.ends_with('\n') {
                        task.task_query.push('\n');
                    }

                    task.task_query.push_str(&result);
                }
            } else {
                tx.send(Event::answer(format!("\n\n{result}")).raw_task_info(0, tool_call_id))?;
            }
        }
    }

    // launching an Agent task pool
    if !tasks_list.is_empty() {
        // remove broken dependencies:
        let active_ids: HashSet<i64> = tasks_list.iter().map(|task| task.task_id).collect();
        for task in tasks_list.iter_mut() {
            task.depend_tasks.retain(|id| active_ids.contains(id));
        }

        // send tool calls to client:
        if let Some(msg) = (&*messages.lock().await).messages.last()
            && msg.role.is_assistant()
        {
            tx.send(Event::start(&msg.tool_calls))?;
        }

        // delegate tasks:
        let tasks_len = tasks_list.len();
        let tasks = Tasks::new(session, messages);

        // collect tasks:
        let mut running = vec![];
        {
            let mut lock = tasks.lock().await;

            for task in tasks_list {
                if task.depend_tasks.is_empty() {
                    running.push(task.task_id);
                }

                lock.pending
                    .insert(task.task_id, Task::new(tx.clone(), tasks.clone(), task));
            }
        };

        // spawning tasks:
        info!("Spawning agent tasks ({tasks_len})");
        for task_id in running {
            handle_task(task_id, tx.clone(), tasks.clone()).await;
        }
    } else {
        tx.send(Event::finish())?;
        info!("The user request was processed without agent tasks");

        // save messages to database:
        let to_save = messages.lock().await.slice(-1);
        session.lock().await.write_messages(to_save).await?;
    }

    Ok(())
}

/// Handles the agent task or pendings it
#[async_recursion]
#[log(skip_all, fields(tid))]
pub async fn handle_task(tid: i64, tx: Sender<Bytes>, tasks: Arc<Mutex<Tasks>>) {
    let mut lock = tasks.lock().await;
    let Some(task) = lock.pending.remove(&tid) else {
        return;
    };

    let tx = tx.clone();
    let tasks = tasks.clone();

    // handle agent task:
    let messages = lock.messages.clone();
    let current = Span::current();
    let child = tokio::spawn(
        async move {
            let session = tasks.lock().await.session.clone();

            if let Err(e) = handle_agent(
                task.agent.clone(),
                session,
                messages,
                tx.clone(),
                task.clone(),
            )
            .await
            {
                error!("{e}");
                // send error to client
                task.tx
                    .send(Event::error(str!("{e}")).task_info(task.info()))
                    .ok();

                // guarantee that client will receive the task closure
                task.tx.send(Event::finish().task_info(task.info())).ok();

                task.finish_branch().await;
            }
        }
        .instrument(current),
    );

    lock.working.insert(tid, arc!(child));
}

/// Handles the agent task
#[log(skip_all, fields(agent = %agent_name, skills = %task.skills.join(",")))]
pub async fn handle_agent(
    agent_name: String,
    session: Arc<Mutex<Session>>,
    messages: Arc<Mutex<Messages>>,
    tx: Sender<Bytes>,
    task: Task,
) -> Result<()> {
    let arc_name = arc!(task.agent.clone());

    // 1. Проверка агента на существование
    let (sock_path, prompt, _skills) = match Manager::ensure_agent(&arc_name).await {
        Ok(Some(ops)) => ops,
        _ => {
            return Err(str!("Agent `{}` is not available or failed to start", task.agent).into());
        }
    };

    // 2. Получение инструментов через IPC
    let client = Client::ipc(&sock_path.to_string_lossy());
    let response = client
        .post("/tools/list")
        .header("Content-Type", "application/json")
        .json(&json!({ "skills": task.skills }))
        .send()
        .await?;
    let tools = response.json::<Vec<anylm::Tool>>().await?;

    // Логирование и отправка события начала работы
    let log_query = task
        .query
        .chars()
        .take(40)
        .collect::<String>()
        .trim_end_matches(".")
        .replace("\n", "\\n");

    info!("Handling `{}` agent: \"{log_query}...\"", task.agent);
    tx.send(
        Event::think(str!(
            "**Handling `{}` agent:** *\"{log_query}...\"*",
            task.agent
        ))
        .task_info(task.info()),
    )
    .ok();

    let ai_conf = &Settings::get().assistant;
    let options = ai_conf.completions.clone();

    // 3. Формирование локального контекста для генерации
    let system_pr = system_prompt(&session.lock().await.info, &ai_conf);
    let agent_messages = Messages::new()
        .system(vec![
            system_pr.into(),
            prompt.trim().into(),
        ])
        .assistant(
            task.context()
                .await
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
            vec![],
        )
        .user(vec![
            str!("{prompt}\n\n{query}",
                prompt = "For the following request, you MUST use the provided tools and MUST NOT answer from your own knowledge. If no suitable tool is available, return an error explaining that the required tool does not exist. Never invent or assume tools that were not provided.",
                query = task.query
            ).into()
        ])
        .wrap();

    let mut tool_calls = vec![];
    let mut retry_count = 0;
    let max_retries = Settings::get().assistant.max_retries.max(1) as usize;

    // --- ОСНОВНОЙ ЦИКЛ ВЗАИМОДЕЙСТВИЯ С LLM ---
    loop {
        tool_calls = vec![];
        let mut text_response = str!();

        let response_res = Completions::try_from(options.clone())?
            .tools(tools.clone())
            .send(agent_messages.clone())
            .await;

        match response_res {
            Ok(mut response) => {
                let mut chunk_error = None;
                while let Some(chunk) = response.next().await {
                    match chunk {
                        Ok(AiChunk::Text(text_part)) => {
                            text_response.push_str(&text_part);
                            tx.send(Event::answer(text_part).task_info(task.info()))?;
                        }
                        Ok(AiChunk::Tool(tool_call)) => {
                            tool_calls.push(tool_call);
                        }
                        Err(e) => {
                            chunk_error = Some(e);
                            break;
                        }
                    }
                }

                // Самовосстановление при ошибке стрима
                if let Some(err) = chunk_error {
                    retry_count += 1;
                    if retry_count < max_retries {
                        warn!(
                            "Error reading stream from agent `{}`. Retrying ({retry_count}/{max_retries}): {err}",
                            task.agent
                        );
                        tx.send(
                            Event::think(str!(
                                "Stream error. Healing and retrying `{}` agent execution...",
                                task.agent
                            ))
                            .task_info(task.info()),
                        )
                        .ok();
                        agent_messages.lock().await.add_user(vec![
                            format!("An error occurred during output generation: {err}. Please try again and complete the task using the available tools.").into()
                        ]);
                        continue;
                    } else {
                        return Err(str!(
                            "Agent `{}` failed after stream error: {err}",
                            task.agent
                        )
                        .into());
                    }
                }

                // Самовосстановление при пустом ответе без вызова инструментов
                if tool_calls.is_empty() && text_response.trim().is_empty() {
                    retry_count += 1;
                    if retry_count < max_retries {
                        warn!(
                            "Agent `{}` returned empty response and no tool calls. Retrying ({retry_count}/{max_retries})...",
                            task.agent
                        );
                        tx.send(Event::think(str!("Agent `{}` returned empty response. Self-healing task execution...", task.agent)).task_info(task.info())).ok();
                        agent_messages.lock().await.add_user(vec![
                            "You did not call any tools. Please execute the requested task using the available tools now.".into()
                        ]);
                        continue;
                    } else {
                        return Err(str!(
                            "Agent `{}` failed to execute task after {} retries: empty output",
                            task.agent,
                            max_retries
                        )
                        .into());
                    }
                }
            }
            Err(e) => {
                retry_count += 1;
                if retry_count < max_retries {
                    warn!(
                        "Failed to send request to Completions for agent `{}`. Retrying ({retry_count}/{max_retries}): {e}",
                        task.agent
                    );
                    tx.send(
                        Event::think(str!(
                            "Request error. Healing and retrying `{}` agent execution...",
                            task.agent
                        ))
                        .task_info(task.info()),
                    )
                    .ok();
                    agent_messages.lock().await.add_user(vec![
                        format!("Failed to process request due to error: {e}. Please attempt to execute the task again using tools.").into()
                    ]);
                    continue;
                } else {
                    return Err(str!(
                        "Agent `{}` failed sending completions request: {e}",
                        task.agent
                    )
                    .into());
                }
            }
        }

        // Если инструментов для вызова нет — выходим из цикла генерации
        if tool_calls.is_empty() {
            break;
        }

        // --- ПАРАЛЛЕЛЬНОЕ ВЫПОЛНЕНИЕ ВЫЗОВОВ ИНСТРУМЕНТОВ ---
        let mut workers = JoinSet::new();

        for tool_call in tool_calls {
            let client = client.clone();
            let sock_path = sock_path.clone();
            let arc_name = arc_name.clone();
            let tx = tx.clone();
            let task = task.clone();

            workers.spawn(async move {
                let func = tool_call.func;
                let log_json = func.json_str.replace("\n", "\\n");
                info!("Calling `{}.{}` tool: {log_json}", task.agent, func.name);

                tx.send(
                    Event::think(str!(
                        "Calling `{} -> {}` tool: {log_json}",
                        task.agent,
                        func.name
                    ))
                    .task_info(task.info()),
                )
                .ok();

                let request_path = format!("/tools/call/{}", func.name);

                // Исправление E0277/E0308: явно маппим ошибку парсинга в Box
                let request_body = func.parse_args::<JsonValue>()?;

                // Отправка запроса на сервер агента
                let mut response = client
                    .post(&request_path)
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .stream::<Event>()
                    .await;

                // Тактический рестарт
                if response.is_err() {
                    warn!(
                        "Agent `{}` didn't respond. Attempting tactical restart...",
                        task.agent
                    );
                    tx.send(
                        Event::think(str!(
                            "Connection lost. Restarting `{}` agent...",
                            task.agent
                        ))
                        .task_info(task.info()),
                    )
                    .ok();

                    let _ = Manager::stop(arc_name.clone()).await;

                    if let Ok(Some((_, _, _))) = Manager::ensure_agent(&arc_name).await {
                        response = Client::ipc(&sock_path.to_string_lossy())
                            .post(&request_path)
                            .header("Content-Type", "application/json")
                            .json(&request_body)
                            .stream::<Event>()
                            .await;
                    }
                }

                // Исправление E0277: кастуем ошибку креша в нужный Box тип
                let mut stream = match response {
                    Ok(res) => res,
                    Err(e) => {
                        return Err(str!(
                            "Agent `{}` crashed and failed to recover: {e}",
                            task.agent
                        )
                        .into());
                    }
                };

                let mut full_text = str!();
                // Исправление E0277: маппим ошибку стрима
                while let Some(event) = stream.recv().await? {
                    // TODO: Перехват интерактивных событий подтверждения (Confirmation events)
                    // TODO: Обработка событий ввода доп. данных (User input injects)

                    match event.kind {
                        EventKind::Answer => {
                            full_text.push_str(&event.text);
                            // Исправление E0277: маппим ошибку отправки в tx
                            tx.send(Event::answer(event.text).task_info(task.info()))?;
                        }
                        EventKind::Finish => {}
                        _ => {
                            tx.send(event.task_info(task.info()))?;
                        }
                    }
                }

                // Явно возвращаем Result с сигнатурой Box<dyn...>
                Ok::<String, DynError>(full_text)
            });
        }

        // Сбор результатов по мере их завершения и моментальная запись в истории
        while let Some(worker_result) = workers.join_next().await {
            // Исправление E0282: Явно пишем String тип для full_text, чтобы .into() ниже отработал однозначно
            let full_text: String =
                worker_result.map_err(|e| str!("Worker task panicked: {e}"))??;
            let content_item: Content = full_text.into();

            // Точечно пишем в локальный контекст для продолжения генерации в loop
            agent_messages
                .lock()
                .await
                .push_content(None, content_item.clone());

            // СРАЗУ сохраняем промежуточный этап в глобальную историю сообщений основного чата
            messages
                .lock()
                .await
                .push_content(Some(&task.tool_call_id), content_item);
        }
    }

    // --- ОПТИМИЗИРОВАННОЕ ЗАВЕРШЕНИЕ РАБОТЫ АГЕНТА ---

    // Забираем накопленные результаты без оверхеда на клонирование строк
    let agent_contents = {
        let mut local_lock = agent_messages.lock().await;
        std::mem::take(&mut local_lock.messages)
            .into_iter()
            .filter(|msg| !msg.role.is_assistant() && !msg.role.is_user())
            .flat_map(|msg| msg.content)
            .collect::<Vec<Content>>()
    };

    // Завершаем задачу в клиенте и пуле
    tx.send(Event::finish().task_info(task.info())).ok();
    task.finish(agent_contents).await;

    // --- ЦИКЛ САМОПРОВЕРКИ (Self-Correction Loop) ---
    if task.is_last().await {
        info!("All parallel tasks completed. Launching verification cycle...");

        let verification_msg = Message::user(vec![
            "Check the results of the completed tasks and announce them. \
            If an error occurs or the results do not match the user's expectation, \
            try to run them again with the changed parameters. \
            If everything is completed successfully and the goal is achieved, print the final coherent response for the user.".into(),
        ]);

        let sid = session.lock().await.id;

        if let Err(e) = handle_query(
            sid,
            tx.clone(),
            session.clone(),
            messages.clone(),
            verification_msg,
        )
        .await
        {
            error!("[verification_loop{{sid={sid}}}] Failed to restart query loop: {e}");
            tx.send(Event::error(str!(e))).ok();
        }
    }

    Ok(())
}

/// Generates the system prompt
fn system_prompt(info: &SessionInfo, ai_conf: &AssistantOptions) -> String {
    let now_utc = Utc::now();
    let now_local = now_local(info.timezone);

    ai_conf
        .system_prompt
        .trim()
        .replace(
            "{DATETIME_LOCAL}",
            &now_local
                .format("%A, %B %d, %Y, %I:%M:%S %p %Z")
                .to_string(),
        )
        .replace(
            "{DATETIME_GLOBAL}",
            &now_utc.format("%A, %B %d, %Y, %I:%M:%S %p UTC").to_string(),
        )
        .replace(
            "{CURRENT_PATH}",
            &info
                .current_path
                .clone()
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_default(),
        )
}

/// Returns the session local date time
fn now_local(timezone_m: i16) -> DateTime<FixedOffset> {
    let offset_seconds = (timezone_m as i32) * 60;
    let tz =
        FixedOffset::east_opt(offset_seconds).unwrap_or_else(|| FixedOffset::east_opt(0).unwrap());

    let utc_now = Utc::now();
    utc_now.with_timezone(&tz)
}
