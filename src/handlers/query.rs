use crate::{
    Session, SessionChunk,
    agents::{AgentAction, DelegatedTasks, SummaryResults},
    cache::AgentCache,
    prelude::*,
};
use anylm::{Chunk, Schema};
use atoman::futures::{TryStreamExt, future};
use reqwest::Client;

/// The request POST data
#[derive(Deserialize)]
pub struct QueryData {
    query: String,
    session_id: String,
}

/// Api '/query' handler
pub async fn handle(Json(data): Json<QueryData>) -> impl IntoResponse {
    let session_id = data.session_id.clone();
    let query = data.query.clone();

    let body = Stream::body(move |tx| async move {
        // initialize session:
        let session = Arc::new(Mutex::new(Session::new(session_id, query.clone(), tx)));

        // starting the delegation cycle:
        if let Err(e) = delegate_tasks_cycled(session.clone(), query).await {
            error!("Chain broken: {e}");
            // we inform the user about a critical error if the stream is still alive:
            let mut guard = session.lock().await;
            guard
                .error(e.to_string(), "Critical error when executing the request")
                .await
                .ok();
        }

        // finalizing (writing to the database, caching, closing tags, etc.):
        let mut guard = session.lock().await;
        guard.finalize_success().await.ok();
    });

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/octet-stream")],
        Body::from_stream(body),
    )
        .into_response()
}

/// Delegates an user query into agents (recursive)
async fn delegate_tasks_cycled(session: Arc<Mutex<Session>>, query: String) -> Result<()> {
    // delegating and completing tasks:
    delegate_tasks(session.clone(), query).await?;

    // summarizing the results:
    summarize_results(session).await?;

    Ok(())
}

/// Delegates an user query into agents
async fn delegate_tasks(session: Arc<Mutex<Session>>, query: String) -> Result<()> {
    if session.lock().await.is_closed() {
        return Err(Error::ClientDisconnected.into());
    }

    // caching logic:
    let query_words = AgentCache::to_words(&query);
    let mut cached = None;

    if Settings::get().agents.caching {
        for agent in Agents::get_all().await.iter() {
            if agent.cache.compare(&query_words[..]).await.unwrap_or(false) {
                cached = Some(agent.manifest.agent.name.clone());
                break;
            }
        }
    }

    let response: DelegatedTasks = if let Some(name) = cached {
        info!("Used cached data to handle {name} agent");
        DelegatedTasks::from_cached_agent(name, query)
    } else {
        // preparing the prompta for the scheduler:
        let prompt = utils::read_prompt("delegate-query")
            .await?
            .replace("{AGENTS}", &Agents::list().await.join("\n"));

        let history = session.lock().await.results().clone();
        let mut request = utils::completions()
            .await?
            .assistant_message(history.into_iter().map(|item| item.into()).collect())
            .system_message(vec![prompt.into()])
            .user_message(vec![query.into()])
            .schema(
                Schema::object("response format").required_property(
                    "tasks",
                    Schema::array("Agent task groups").items(
                        Schema::array("Agent tasks group").items(
                            Schema::object("The agent task")
                                .required_property("name", Schema::string("Agent name"))
                                .required_property("query", Schema::string("Query to agent"))
                                .required_property(
                                    "keys",
                                    Schema::array("Keywords").items(Schema::string("")),
                                ),
                        ),
                    ),
                ),
            );

        let mut resp = request.send().await?;
        let mut buffer = str!();
        while let Some(chunk) = resp.next().await {
            if let Chunk::Text(text) = chunk? {
                buffer.push_str(&text);
            }
        }
        json::from_str(&buffer).map_err(|e| fmt!("Incorrect response format: {e}"))?
    };

    // completing tasks:
    if let Some(mut tasks) = response.tasks
        && !tasks.is_empty()
    {
        // caching of simple queries:
        if tasks.len() == 1 && tasks[0].len() == 1 {
            let task = &mut tasks[0][0];
            if let Some(keys) = task.keys.take()
                && let Some(agent) = Agents::get(&task.name).await
            {
                agent.cache.write_keys(keys).await?;
            }
        }

        for group in tasks {
            if session.lock().await.is_closed() {
                break;
            }

            let handles: Vec<_> = group
                .into_iter()
                .map(|task| {
                    let session = session.clone();
                    async move {
                        if let Err(e) = handle_agent(session.clone(), task.name, task.query).await {
                            let mut guard = session.lock().await;
                            guard
                                .error(e.to_string(), "Agent failed to process task")
                                .await
                                .ok();
                        }
                    }
                })
                .collect();

            future::join_all(handles).await;
        }
    }

    Ok(())
}

/// Handles agent query
async fn handle_agent(session: Arc<Mutex<Session>>, agent: String, query: String) -> Result<()> {
    let actions = handle_query(session.clone(), &agent, &query).await?;

    if session.lock().await.is_closed() {
        return Err(Error::ClientDisconnected.into());
    }

    for action in actions {
        handle_action(session.clone(), &agent, &action.name, action.data).await?;
    }

    Ok(())
}

/// Handles query by LM
async fn handle_query(
    session: Arc<Mutex<Session>>,
    agent: &str,
    query: &str,
) -> Result<Vec<AgentAction>> {
    info!("⏳ Processing query to {agent} agent");

    // displaying the status to the user:
    session
        .lock()
        .await
        .think(fmt!(" Processing ({agent}): {query}\n"))
        .await?;

    let prompt = utils::read_prompt("handle-query")
        .await?
        .replace("{EXAMPLES}", &Agents::exmpls(agent).await.join("\n"));

    let history = session.lock().await.results().clone();
    let mut request = utils::completions()
        .await?
        .system_message(vec![
            utils::read_prompt("assistant-character").await?.into(),
        ])
        .assistant_message(history.into_iter().map(|item| item.into()).collect())
        .system_message(vec![prompt.into()])
        .user_message(vec![query.into()])
        .tools(Agents::tools(agent).await);

    let mut response = request.send().await?;
    let mut actions = vec![];

    while let Some(chunk) = response.next().await {
        match chunk? {
            Chunk::Tool(name, data) => actions.push(AgentAction {
                name,
                data: json::from_str(&data)?,
            }),
            _ => {}
        }
    }

    Ok(actions)
}

/// Handles action call
async fn handle_action(
    session: Arc<Mutex<Session>>,
    agent: &str,
    action: &str,
    data: JsonValue,
) -> Result<()> {
    // logging the action in the Thinking block:
    {
        let mut guard = session.lock().await;
        let data_str = json::to_string(&data).unwrap_or_default();
        guard
            .think(fmt!(" Handling ({agent}): /{action} {data_str}\n"))
            .await?;
    }

    // request to the Agent's API:
    let port = Settings::get().server.port;
    let response = Client::new()
        .post(fmt!("http://127.0.0.1:{port}/call/{agent}/{action}"))
        .json(&data)
        .send()
        .await?;

    let stream = response.bytes_stream().map_err(|e| {
        let err: Box<dyn std::error::Error + Send + Sync> = Box::new(e);
        err
    });

    // streaming chunks from the agent directly into the session:
    let mut reader = Stream::read::<SessionChunk, _>(stream);

    while let Some(chunk) = reader.read().await? {
        let mut guard = session.lock().await;
        guard.push(chunk).await?;
    }

    Ok(())
}

/// Summarizes the execution results
async fn summarize_results(session: Arc<Mutex<Session>>) -> Result<()> {
    let history = session.lock().await.results().clone();

    let mut response = utils::completions()
        .await?
        .system_message(vec![
            utils::read_prompt("assistant-character").await?.into(),
        ])
        .assistant_message(vec![history.join("\n").into()])
        .system_message(vec![utils::read_prompt("summary-results").await?.into()])
        .schema(
            Schema::object("summary")
                .required_property("answer", Schema::string("Final answer"))
                .required_property("context", Schema::string("Full context")),
        )
        .send()
        .await?;

    let mut buffer = str!();
    while let Some(chunk) = response.next().await {
        if let Chunk::Text(text) = chunk? {
            buffer.push_str(&text);
        }
    }

    let SummaryResults { answer, .. } = json::from_str(&buffer)?;

    // output of the final answer directly:
    let mut guard = session.lock().await;
    let dur = guard.exec_time() as f64 / 1000.0;
    guard.answer(fmt!("{answer}")).await?;
    guard
        .info(&fmt!("\n\n[EOF] Execution time: {dur} sec."))
        .await?;

    Ok(())
}
