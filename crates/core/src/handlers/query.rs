use crate::{Runtime, Session, manager::*, prelude::*};
use anylm::{AiChunk, Completions, Message, Messages, ToolCall};
use chrono::FixedOffset;
use ovsy_share::{
    AgentTask, Event, EventKind, HandleQuery, SessionInfo, settings::AssistantOptions,
};
use std::collections::HashSet;

/// API: The user message handler
#[log(skip_all, fields(sid = %sid.0))]
pub async fn handle_query(sid: Paths<SessionId>, data: Json<HandleQuery>) -> Response {
    let session_id = sid.0;
    let HandleQuery { message } = data.0;

    let current = Span::current();

    Response::ok().stream(move |tx| {
        let current2 = current.clone();
        async move {
            if let Err(e) = handle(session_id, tx.clone(), message)
                .instrument(current2)
                .await
            {
                error!("{e}");
                tx.send(Event::error(str!(e))).ok();
            }
        }
        .instrument(current)
    })
}

/// Handles the user query with self-healing on planning/generation level
async fn handle(session_id: SessionId, tx: Sender<Bytes>, message: Message) -> Result<()> {
    let ai_conf = &Settings::get().assistant;
    let options = ai_conf.completions.clone();

    info!("Initialized the user request processing");

    // init session & read messages:
    let Some(session) = Session::get(&session_id) else {
        return Err(Error::UnknownSessionId(session_id).into());
    };
    let db_messages = session.lock().await.read_messages().await?;

    // prepare messages:
    let messages = Messages::from(db_messages)
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
                    "handle_agent" => match tool_call.parse_args::<AgentTask>() {
                        Ok(task) => {
                            tasks_list.push(task.sess_id(session_id).tool_id(tool_call.id));
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
            task.wait_for.retain(|id| active_ids.contains(id));
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
                if task.wait_for.is_empty() {
                    running.push(task.task_id);
                }

                lock.pending.insert(task.task_id, task);
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
#[log(skip_all, fields(tid = %task_id))]
pub async fn handle_task(task_id: i64, tx: Sender<Bytes>, tasks: Arc<Mutex<Tasks>>) {
    let mut lock = tasks.lock().await;
    let Some(task) = lock.pending.remove(&task_id) else {
        return;
    };

    let tx = tx.clone();
    let tasks = tasks.clone();

    // handle agent task:
    let messages = lock.messages.clone();
    let current = Span::current();
    let child = tokio::spawn(
        async move {
            let handle = Task {
                task_info: arc!(task),
                tasks: tasks.clone(),
                tx: tx.clone(),
            };

            let session_id = handle.task_info.session_id;
            let session = tasks.lock().await.session.clone();

            if let Err(e) = handle_agent(
                handle.task_info.agent_name.clone(),
                session,
                messages,
                tx.clone(),
                handle.clone(),
            )
            .await
            {
                error!("[handle_agent{{sid={session_id}}} -> handle] {e}");
                // send error to client
                handle
                    .tx
                    .send(Event::error(str!("{e}")).task_info(&handle.task_info))
                    .ok();

                // guarantee that client will receive the task closure
                handle
                    .tx
                    .send(Event::finish().task_info(&handle.task_info))
                    .ok();

                handle.finish_branch().await;
            }
        }
        .instrument(current),
    );

    lock.working.insert(task_id, arc!(child));
}

/// Handles the AI-agent
#[log(skip_all, fields(agent = %agent_name, skills = %task.task_info.agent_skills.join(",")))]
pub async fn handle_agent(
    agent_name: String,
    session: Arc<Mutex<Session>>,
    messages: Arc<Mutex<Messages>>,
    tx: Sender<Bytes>,
    task: Task,
) -> Result<()> {
    let task_info = task.task_info.clone();
    let arc_name = arc!(task_info.agent_name.clone());

    // check agent for existence:
    let (sock_path, prompt, _skills) = match Manager::ensure_agent(&arc_name).await {
        Ok(Some(ops)) => ops,
        _ => {
            return Err(str!(
                "Agent `{}` is not available or failed to start",
                task_info.agent_name
            )
            .into());
        }
    };
    warn!("{:#?}", task.task_info);
    let req_skills = &task.task_info.agent_skills;

    // receive agent tools:
    let client = Client::ipc(&sock_path.to_string_lossy());
    let response = client
        .post("/tools/list")
        .header("Content-Type", "application/json")
        .json(&json!({ "skills": req_skills }))
        .send()
        .await?;
    let tools = response.json::<Vec<anylm::Tool>>().await?;

    // logging to thinking block:
    let log_query = task_info
        .task_query
        .chars()
        .take(40)
        .collect::<String>()
        .trim_end_matches(".")
        .replace("\n", "\\n");
    info!(
        "Handling `{}` agent: \"{log_query}...\"",
        task_info.agent_name
    );
    tx.send(
        Event::think(str!(
            "**Handling `{}` agent:** *\"{log_query}...\"*",
            task_info.agent_name
        ))
        .task_info(&task_info),
    )
    .ok();
    drop(log_query);

    let ai_conf = &Settings::get().assistant;
    let options = ai_conf.completions.clone();

    // prepare messages:
    let agent_messages = Messages::new()
        .system(vec![
            system_prompt(&session.lock().await.info, &ai_conf).into(),
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
        .user(vec![task_info.task_query.clone().into()])
        .wrap();

    let mut tool_calls = vec![];
    let mut retry_count = 0;

    let max_retries = Settings::get().assistant.max_retries.max(1) as usize;

    // self-healing generation cycle in case of empty responses or API errors
    loop {
        tool_calls.clear();
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

                if let Some(err) = chunk_error {
                    retry_count += 1;
                    if retry_count < max_retries {
                        warn!(
                            "Error reading stream from agent `{}`. Retrying ({retry_count}/{max_retries}): {err}",
                            task_info.agent_name
                        );
                        tx.send(
                            Event::think(str!(
                                "Stream error. Healing and retrying `{}` agent execution...",
                                task_info.agent_name
                            ))
                            .task_info(&task_info),
                        )
                        .ok();

                        agent_messages.lock().await.add_user(vec![
                            format!("An error occurred during output generation: {err}. Please try again and complete the task using the available tools.").into()
                        ]);
                        continue;
                    } else {
                        return Err(str!(
                            "Agent `{}` failed after stream error: {err}",
                            task_info.agent_name
                        )
                        .into());
                    }
                }

                // if LLM returned an empty text and there are no tool calls:
                if tool_calls.is_empty() && text_response.trim().is_empty() {
                    retry_count += 1;
                    if retry_count < max_retries {
                        warn!(
                            "Agent `{}` returned empty response and no tool calls. Retrying ({retry_count}/{max_retries})...",
                            task_info.agent_name
                        );
                        tx.send(
                            Event::think(str!(
                                "Agent `{}` returned empty response. Self-healing task execution...",
                                task_info.agent_name
                            ))
                            .task_info(&task_info),
                        )
                        .ok();

                        agent_messages.lock().await.add_user(vec![
                            "You did not call any tools. Please execute the requested task using the available tools now.".into()
                        ]);
                        continue;
                    } else {
                        return Err(str!(
                            "Agent `{}` failed to execute task after {} retries: empty output",
                            task_info.agent_name,
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
                        task_info.agent_name
                    );
                    tx.send(
                        Event::think(str!(
                            "Request error. Healing and retrying `{}` agent execution...",
                            task_info.agent_name
                        ))
                        .task_info(&task_info),
                    )
                    .ok();

                    agent_messages.lock().await.add_user(vec![
                        format!("Failed to process request due to error: {e}. Please attempt to execute the task again using tools.").into()
                    ]);
                    continue;
                } else {
                    return Err(str!(
                        "Agent `{}` failed sending completions request: {e}",
                        task_info.agent_name
                    )
                    .into());
                }
            }
        }

        break;
    }

    // handle tools:
    for ToolCall { func, .. } in tool_calls {
        let log_json = func.json_str.replace("\n", "\\n");
        info!(
            "Calling `{}.{}` tool: {log_json}",
            task_info.agent_name, func.name
        );
        tx.send(
            Event::think(str!(
                "Calling `{} -> {}` tool: {log_json}",
                task_info.agent_name,
                func.name
            ))
            .task_info(&task_info),
        )
        .ok();
        drop(log_json);

        let request_path = str!("/tools/call/{}", func.name);
        let request_body = func.parse_args::<JsonValue>()?;

        // send to agent server:
        let mut response = client
            .post(&request_path)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .stream::<Event>()
            .await;

        if response.is_err() {
            warn!(
                "Agent `{}` didn't respond. Attempting tactical restart...",
                task_info.agent_name
            );
            tx.send(
                Event::think(str!(
                    "Connection lost. Restarting `{}` agent...",
                    task_info.agent_name
                ))
                .task_info(&task_info),
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

        let mut stream = match response {
            Ok(res) => res,
            Err(e) => {
                return Err(str!(
                    "Agent `{}` crashed and failed to recover: {e}",
                    task_info.agent_name,
                )
                .into());
            }
        };

        // read stream chunks and accumulate tool outputs to full_text:
        let mut full_text = str!();
        info!("Trying to receive chunks..");
        while let Some(event) = stream.recv().await? {
            match event.kind {
                EventKind::Answer => full_text.push_str(&event.text),
                EventKind::Finish => {}
                _ => tx.send(event.task_info(&task_info))?,
            }
        }

        // Save tool's accumulated result into message history:
        agent_messages.lock().await.push_content(None, full_text);
    }

    // control request to LLM (Response Synthesis) ---
    agent_messages
        .lock()
        .await
        .add_user(vec![
            "Based on the steps performed above and the data received, prepare a final coherent response for the user.".into()
        ]);

    info!(
        "Sending synthesising final request for agent `{}`",
        task_info.agent_name
    );
    let mut final_response = Completions::try_from(options)?
        .send(agent_messages.clone())
        .await?;

    while let Some(chunk) = final_response.next().await {
        match chunk? {
            AiChunk::Text(text_part) => {
                // stream the final human-readable response to the client
                tx.send(Event::answer(text_part).task_info(&task_info))?;
            }
            AiChunk::Tool(_) => {
                // ignoring tool calls at the synthesis stage
            }
        }
    }

    let mut lock = messages.lock().await;
    let final_content = {
        let lock2 = agent_messages.lock().await;
        lock2
            .messages
            .last()
            .map(|m| {
                for cnt in m.content.clone() {
                    lock.push_content(Some(&task.task_info.tool_call_id), cnt);
                }

                m.content.clone()
            })
            .unwrap_or_default()
    };
    drop(lock);

    // finish agent handling successfully:
    tx.send(Event::finish().task_info(&task.task_info)).ok();

    // finish query cycle:
    if task.is_last().await {
        info!("The last task was completed, saving the session");
        tx.send(Event::finish().task_info(&task.task_info)).ok();
        tx.send(Event::finish()).ok();

        // save messages to database:
        let to_save = messages.lock().await.slice(-1);
        session.lock().await.write_messages(to_save).await?;
    }

    // finish task:
    task.finish(final_content).await;

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
