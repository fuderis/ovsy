use crate::{manager::AgentHandle, prelude::*};
use anylm::{AiChunk, Completions, Messages, ToolCall};
use ovsy_share::{AgentTask, Chunk, ChunkData, UserQuery, settings::AssistantOptions};
use reqwest::Client;

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

/// API: The user query handler
pub async fn handle(data: Json<UserQuery>) -> Response {
    let body = Stream::body(move |tx| async move {
        if let Err(e) = handle_query(tx.clone(), data.0).await {
            error!("{e}");
            tx.send(Chunk::error(str!("{e}"))).ok();
        }
    });

    Response::ok().stream(body)
}

/// Handles the user query
pub(crate) async fn handle_query(tx: Sender, data: UserQuery) -> Result<()> {
    let ai_conf = &Settings::get().assistant;
    let options = ai_conf.completions.clone();

    // prepare messages:
    let messages = Messages::new()
        .system(vec![
            system_prompt(&ai_conf).into(),
            ai_conf
                .assist_prompt
                .replace("{AGENTS_LIST}", &Manager::agents_list_doc().await)
                .into(),
        ])
        .messages(data.messages)
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
                tasks.push(task.tool_id(tool_call.id));
            }
        }
    }

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
        AgentHandle::handle_all(tx.clone(), tasks).await;
    } else {
        tx.send(Chunk::finish())?;
    }

    Ok(())
}

/// Handles the AI-agent
pub(crate) async fn handle_agent(tx: Sender, handle: AgentHandle) -> Result<()> {
    let task = handle.task.clone();
    let arc_name = arc!(task.agent_name.clone());

    // check agent for exists:
    let (port, prompt, tools) = if let Some(ops) = Manager::agent_options(&arc_name).await {
        ops
    } else {
        let err_msg = str!("Agent `{}` is not available", task.agent_name);
        tx.send(Chunk::error(err_msg).task_info(task.clone_minimal()))
            .ok();
        handle.finish_branch().await;
        return Ok(());
    };

    // logging to thinking block:
    let msg = str!(
        "**Handling `{}`:** *\"{:.40}...\"*",
        task.agent_name,
        task.task_query.trim_end_matches(".")
    );
    info!("{msg}");
    tx.send(Chunk::think(msg).task_info(task.clone_minimal()))
        .ok();

    let ai_conf = &Settings::get().assistant;
    let options = ai_conf.completions.clone();

    // prepare messages:
    let messages = Messages::new()
        .system(vec![system_prompt(&ai_conf).into(), prompt.into()])
        .assistant(
            vec![
                str!(
                    "**Context**: {}",
                    handle
                        .context()
                        .await
                        .iter()
                        .map(|s| s.as_str().to_string())
                        .collect::<Vec<_>>()
                        .join("\n")
                )
                .into(),
            ],
            vec![],
        )
        .user(vec![
            str!("**User Query (handle it)**: {}", task.task_query).into(),
        ])
        .wrap();

    // send request:
    let mut response = Completions::try_from(options)?
        .tools(tools)
        .send(messages)
        .await?;

    // send request:
    let mut full_text = str!();
    let mut tool_calls = vec![];

    // read ai chunks:
    while let Some(chunk) = response.next().await {
        match chunk? {
            AiChunk::Text(text_part) => {
                full_text.push_str(&text_part);
                tx.send(Chunk::answer(text_part).task_info(task.clone_minimal()))?;
            }
            AiChunk::Tool(tool_call) => tool_calls.push(tool_call),
        }
    }

    // handle tools:
    let client = Client::new();
    for ToolCall { func, .. } in tool_calls {
        full_text.push('\n');

        let msg = str!(
            "**Calling tool:** *`{} -> {}`...*\n",
            task.agent_name,
            func.name
        );
        info!("{msg}");
        tx.send(Chunk::think(msg).task_info(task.clone_minimal()))
            .ok();

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
            tx.send(chunk.task_info(task.clone_minimal()))?;
        }
    }

    handle.finish(full_text).await;
    Ok(())
}
