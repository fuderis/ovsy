use crate::{Runtime, Session, manager::*, prelude::*};
use anylm::{AiChunk, Completions, Message, Messages, ToolCall};
use chrono::FixedOffset;
use ovsy_share::{
    AgentTask, Chunk, ChunkData, HandleQuery, SessionInfo, settings::AssistantOptions,
};

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
                tx.send(Chunk::error(str!(e))).ok();
            }
        }
        .instrument(current)
    })
}

/// Handles the user query
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

    // send request:
    let mut response = Completions::try_from(options)?
        .tools(Manager::basic_tools().await)
        .send(messages.clone())
        .await?;

    let mut tasks_list = vec![];
    let mut evals_list = vec![];

    #[derive(Deserialize)]
    struct EvalAction {
        task_id: Option<i64>,
        parameter: Option<String>,
        code: String,
    }

    // read ai chunks:
    while let Some(chunk) = response.next().await {
        match chunk? {
            AiChunk::Text(text_part) => tx.send(Chunk::answer(text_part))?,
            AiChunk::Tool(tool_call) => match tool_call.func.name.as_ref() {
                "handle_agent" => {
                    let task: AgentTask = tool_call.parse_args()?;
                    tasks_list.push(task.sess_id(session_id).tool_id(tool_call.id));
                }

                "javascript_eval" => {
                    let eval: EvalAction = tool_call.parse_args()?;
                    evals_list.push((tool_call.id, eval));
                }

                _ => {}
            },
        }
    }

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
                tx.send(Chunk::answer(format!("\n\n{result}")).task_info(AgentTask {
                    task_id: 0,
                    session_id,
                    tool_call_id,
                    task_query: str!(),
                    wait_for: set![],
                    agent_name: str!("js_eval"),
                }))?;
            }
        }
    }

    // save messages to db:
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
            tx.send(Chunk::tools(msg.tool_calls.clone()))?;
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
        tx.send(Chunk::finish())?;
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
                task: arc!(task),
                tasks: tasks.clone(),
                tx: tx.clone(),
            };

            let session_id = handle.task.session_id;
            let session = tasks.lock().await.session.clone();
            if let Err(e) = handle_agent(
                handle.task.agent_name.clone(),
                session,
                messages,
                tx,
                handle.clone(),
            )
            .await
            {
                error!("[handle_agent{{sid={session_id}}} -> handle] {e}");
                handle.tx.send(Chunk::error(str!("{e}"))).ok();
                handle.finish_branch().await;
            }
        }
        .instrument(current),
    );

    lock.working.insert(task_id, arc!(child));
}

/// Handles the AI-agent
#[log(skip_all, fields(agent = %agent_name))]
pub async fn handle_agent(
    agent_name: String,
    session: Arc<Mutex<Session>>,
    messages: Arc<Mutex<Messages>>,
    tx: Sender<Bytes>,
    task: Task,
) -> Result<()> {
    let task_info = task.task.clone();
    let arc_name = arc!(task_info.agent_name.clone());

    // check agent for exists:
    let (sock_path, prompt, tools) = match Manager::ensure_agent(&arc_name).await {
        Ok(Some(ops)) => ops,
        _ => {
            return Err(str!(
                "Agent `{}` is not available or failed to start",
                task_info.agent_name
            )
            .into());
        }
    };

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
        Chunk::think(str!(
            "**Handling `{}` agent:** *\"{log_query}...\"*",
            task_info.agent_name
        ))
        .task_info(task_info.clone_minimal()),
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

    // send request:
    let mut response = Completions::try_from(options)?
        .tools(tools)
        .send(agent_messages.clone())
        .await?;

    // send request:
    let mut tool_calls = vec![];

    // read ai chunks:
    while let Some(chunk) = response.next().await {
        match chunk? {
            AiChunk::Text(text_part) => {
                tx.send(Chunk::answer(text_part).task_info(task_info.clone_minimal()))?
            }
            AiChunk::Tool(tool_call) => tool_calls.push(tool_call),
        }
    }

    // handle tools:
    let client = Client::ipc(&sock_path.to_string_lossy());
    for ToolCall { func, .. } in tool_calls {
        let log_json = func.json_str.replace("\n", "\\n");
        info!(
            "Calling `{}.{}` tool: {log_json}",
            task_info.agent_name, func.name
        );
        tx.send(
            Chunk::think(str!(
                "Calling `{} -> {}` tool: {log_json}",
                task_info.agent_name,
                func.name
            ))
            .task_info(task_info.clone_minimal()),
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
            .stream::<Chunk>()
            .await;

        if response.is_err() {
            warn!(
                "Agent `{}` didn't respond. Attempting tactical restart...",
                task_info.agent_name
            );
            tx.send(
                Chunk::think(str!(
                    "Connection lost. Restarting `{}` agent...",
                    task_info.agent_name
                ))
                .task_info(task_info.clone_minimal()),
            )
            .ok();

            let _ = Manager::stop(arc_name.clone()).await;

            if let Ok(Some((_, _, _))) = Manager::ensure_agent(&arc_name).await {
                response = Client::ipc(&sock_path.to_string_lossy())
                    .post(&request_path)
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .stream::<Chunk>()
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

        // read stream chunks:
        let mut full_text = str!();
        info!("Trying to receive chunks..");
        while let Some(chunk) = stream.recv().await? {
            info!("{chunk:?}");

            match &chunk {
                Chunk {
                    data: ChunkData::Answer(answer),
                    ..
                } => {
                    full_text.push_str(answer);
                }
                _ => {}
            }
            tx.send(chunk.task_info(task_info.clone_minimal()))?;
        }

        agent_messages.lock().await.push_content(None, full_text);
    }

    let mut lock = messages.lock().await;
    let final_content = {
        let lock2 = agent_messages.lock().await;
        lock2
            .messages
            .last()
            .map(|m| {
                for cnt in m.content.clone() {
                    lock.push_content(Some(&task.task.tool_call_id), cnt);
                }

                m.content.clone()
            })
            .unwrap_or_default()
    };
    drop(lock);

    // finish agent handling:
    tx.send(Chunk::finish().task_info(task.task.clone_minimal()))
        .ok();

    // finish query cycle:
    if task.is_last().await {
        info!("The last task was completed, saving the session");
        tx.send(Chunk::finish()).ok();

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
