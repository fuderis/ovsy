use crate::{manager::*, prelude::*};
use anylm::{AiChunk, Completions, Message, Messages, ToolCall};
use ovsy_share::{AgentTask, Chunk, ChunkData, HandleQuery, settings::AssistantOptions};
use reqwest::Client;

/// API: The user message handler
pub async fn handle(data: Json<HandleQuery>) -> Response {
    let session_id = data.0.session_id;

    let body = Stream::body(move |tx| async move {
        if let Err(e) = handle_query(session_id, tx.clone(), data.0.message).await {
            error!("[handle_query{{sid={session_id}}}] {e}");
            tx.send(Chunk::error(str!(e))).ok();
        }
    });

    Response::ok().stream(body)
}

/// Handles the user query
#[log(skip_all, fields(sid = %session_id))]
async fn handle_query(session_id: SessionID, tx: Sender, message: Message) -> Result<()> {
    let ai_conf = &Settings::get().assistant;
    let options = ai_conf.completions.clone();

    info!("Initialized user request processing");

    // init session & read messages:
    let session = Session::new(session_id).await?;
    let db_messages = session.read_messages().await?;

    info!("{db_messages:#?}");

    // prepare messages:
    let messages = Messages::from(db_messages)
        .system(vec![
            system_prompt(&ai_conf).into(),
            ai_conf
                .assist_prompt
                .replace("{AGENTS_LIST}", &Manager::agents_list_doc().await)
                .into(),
        ])
        .message(message)
        .wrap();

    // send request:
    let mut response = Completions::try_from(options)?
        .tool(Manager::task_tool().await)
        .send(messages.clone())
        .await?;

    let mut tasks = vec![];

    // read ai chunks:
    while let Some(chunk) = response.next().await {
        match chunk? {
            AiChunk::Text(text_part) => tx.send(Chunk::answer(text_part))?,
            AiChunk::Tool(tool_call) => {
                let task: AgentTask = tool_call.parse_args()?;
                tasks.push(task.sess_id(session_id).tool_id(tool_call.id));
            }
        }
    }

    // save messages to db:
    if !tasks.is_empty() {
        // remove broken dependencies:
        let active_ids: HashSet<i64> = tasks.iter().map(|task| task.task_id).collect();
        for task in tasks.iter_mut() {
            task.wait_for.retain(|id| active_ids.contains(id));
        }

        // send tool calls to client:
        if let Some(msg) = (&*messages.lock().await).messages.last()
            && msg.role.is_assistant()
        {
            tx.send(Chunk::tools(msg.tool_calls.clone()))?;
        }

        // delegate tasks:
        handle_agents(tx.clone(), session, messages, tasks).await;
    } else {
        tx.send(Chunk::finish())?;
        info!("The user request was processed without agent tasks");

        // save messages to database:
        let to_save = messages.lock().await.slice(-1);
        session.write_messages(to_save).await?;
    }

    Ok(())
}

/// Handles the all agent tasks
async fn handle_agents(
    tx: Sender,
    session: Session,
    messages: Arc<Mutex<Messages>>,
    tasks_list: Vec<AgentTask>,
) {
    if tasks_list.is_empty() {
        return;
    }

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

    // running tasks:
    for id in running {
        Task::handle(tx.clone(), id, tasks.clone()).await;
    }
}

/// Handles the AI-agent
#[log(skip_all, fields(sid = %session_id, agent = %agent_name))]
pub async fn handle_agent(
    session_id: SessionID,
    session: Session,
    messages: Arc<Mutex<Messages>>,
    agent_name: String,
    tx: Sender,
    task: Task,
) -> Result<()> {
    let task_info = task.task.clone();
    let arc_name = arc!(task_info.agent_name.clone());

    // check agent for exists:
    let (port, prompt, tools) = if let Some(ops) = Manager::agent_options(&arc_name).await {
        ops
    } else {
        let err_msg = str!("Agent `{}` is not available", task_info.agent_name);
        tx.send(Chunk::error(err_msg).task_info(task_info.clone_minimal()))
            .ok();
        task.finish_branch().await;
        return Ok(());
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
        .system(vec![system_prompt(&ai_conf).into(), prompt.into()])
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
    let client = Client::new();
    for ToolCall { func, .. } in tool_calls {
        let log_json = func.json_str.replace("\n", "\\n");
        info!(
            "Calling `{}.{}` tool: {log_json}",
            task_info.agent_name, func.name
        );
        tx.send(
            Chunk::think(str!(
                "Calling `{}.{}` tool: {log_json}",
                task_info.agent_name,
                func.name
            ))
            .task_info(task_info.clone_minimal()),
        )
        .ok();
        drop(log_json);

        // send to agent server:
        let response = client
            .post(&str!("http://127.0.0.1:{port}/call/{}", func.name))
            .header("Content-Type", "application/json")
            .json(&func.parse_args::<JsonValue>()?)
            .send()
            .await?;

        // init stream reader:
        let bytes_stream = response.bytes_stream().map(|v| v.map_err(Into::into));
        let mut stream = Stream::read::<Chunk>(bytes_stream);

        // read stream chunks:
        let mut full_text = str!();
        while let Some(chunk) = stream.read().await? {
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

    // finish query cicle:
    if task.is_last().await {
        tx.send(Chunk::finish()).ok();

        // save messages to database:
        let to_save = messages.lock().await.slice(-1);
        session.write_messages(to_save).await?;
    }

    // finish task:
    task.finish(final_content).await;
    Ok(())
}

/// Generates the system prompt
fn system_prompt(ai_conf: &AssistantOptions) -> String {
    let now_utc = Utc::now();
    let now_local = Local::now();
    let time_format = "%A, %B %d, %I:%M:%S %p";

    ai_conf
        .system_prompt
        .replace(
            "{DATETIME_LOCAL}",
            &now_local.format(time_format).to_string(),
        )
        .replace(
            "{DATETIME_GLOBAL}",
            &now_utc.format(time_format).to_string(),
        )
}
