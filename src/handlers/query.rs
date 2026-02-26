use crate::{
    SessionLogger,
    agents::{AgentAction, DelegatedTasks},
    cache::AgentCache,
    prelude::*,
};
use anylm::{Chunk, Schema};
use reqwest::Client;

/// The request POST data
#[derive(Deserialize)]
pub struct QueryData {
    query: String,
    session_id: String,
}

/// Api '/query' handler
pub async fn handle(Json(data): Json<QueryData>) -> impl IntoResponse {
    // create query session:
    let session = Arc::new(Mutex::new(
        SessionLogger::new(data.session_id.clone(), &data.query)
            .await
            .map_err(|e| fmt!("Failed to create session: {e}"))
            .unwrap(),
    ));
    let session2 = session.clone();
    let session_clone = session.clone();

    // create stream body:
    let body = Stream::spawn(
        async move |st| {
            delegate_tasks_cycled(st.clone(), session_clone, data.query, 0).await;
        },
        async move |msg| match msg {
            Ok(bytes) => {
                let mut guard = session2.lock().await;
                guard.write(&String::from_utf8_lossy(&bytes)).await.ok();

                Ok(bytes)
            }
            Err(e) => {
                error!("{e}");

                let mut guard = session2.lock().await;
                guard.write(&fmt!("[Error]: {e}")).await.ok();

                Ok(Bytes::from(fmt!(
                    "[Error]: {e}\n\n[Duration]: {} ms\n[EOF]",
                    guard.exec_time()
                )))
            }
        },
    )
    .await;

    (
        StatusCode::OK,
        HeaderMap::from_iter(map! {
            header::CONTENT_TYPE => "application/octet-stream".parse().unwrap()
        }),
        Body::from_stream(body),
    )
        .into_response()
}

/// Delegates an user query into agents (recursive)
async fn delegate_tasks_cycled(
    st: Stream,
    session: Arc<Mutex<SessionLogger>>,
    query: String,
    recurs: usize,
) {
    if let Err(e) = delegate_tasks(st.clone(), session.clone(), query, recurs).await {
        st.send(Err(e)).ok();
    }
}

/// Delegates an user query into agents
async fn delegate_tasks(
    st: Stream,
    session: Arc<Mutex<SessionLogger>>,
    query: String,
    mut recurs: usize,
) -> Result<()> {
    // check recursion & client connection:
    recurs += 1;
    let limit = Settings::get().agents.recurs_limit;
    if limit > 0 && recurs > limit {
        return Err(Error::RecursionLimit.into());
    } else if st.is_closed() {
        return Err(Error::ClientDisconnected.into());
    }

    // delegate tasks:
    let query_words = AgentCache::to_words(&query);
    let mut cached = None;

    // read cached data:
    if Settings::get().agents.caching {
        for agent in Agents::get_all().await.iter() {
            if agent.cache.compare(&query_words[..]).await.unwrap_or(false) {
                let _ = cached.insert(agent.manifest.agent.name.clone());
            }
        }
    }

    // check if cache found:
    let response: DelegatedTasks = if let Some(name) = cached {
        info!("Used cached data to handle {name} agent");
        DelegatedTasks::from_cached_agent(name, query)
    }
    // handle with AI:
    else {
        // read prompt:
        let prompt = utils::read_prompt("delegate-query")
            .await?
            .replace("{AGENTS}", &Agents::list().await.join("\n"));

        // create query:
        let history = session.lock().await.results().clone();
        let mut request = utils::completions()?
            .assistant_message(history.into_iter().map(|item| item.into()).collect())
            .system_message(vec![prompt.into()])
            .user_message(vec![query.into()])
            .schema(
                Schema::object("response format")
                    .required_property(
                        "tasks",
                        Schema::array("Agents tasks").items(
                            Schema::object("The agent task")
                                .required_property("name", Schema::string("The agent name"))
                                .required_property(
                                    "query",
                                    Schema::string("The query to agent (don't shorten it)"),
                                )
                                .required_property(
                                    "keys",
                                    Schema::array("The query basic keywords")
                                        .items(Schema::string("")),
                                ),
                        ),
                    )
                    .optional_property(
                        "say",
                        Schema::string("The assistant execution progress answer"),
                    ),
            );

        // send request:
        let mut response = request.send().await?;

        // read response stream:
        let mut buffer = str!();
        while let Some(chunk) = response.next().await {
            if let Chunk::Text(text) = chunk? {
                buffer.push_str(&text);
            }
        }

        // parse response:
        json::from_str(&buffer)?
    };

    // handle tasks:
    if let Some(mut tasks) = response.tasks
        && !tasks.is_empty()
    {
        // cache results:
        if tasks.len() == 1 {
            let task = &mut tasks[0];
            if let Some(keys) = task.keys.take()
                && let Some(agent) = Agents::get(&task.name).await
            {
                agent.cache.write_keys(keys).await?;
            }
        }

        // handle agents step by step:
        for task in tasks {
            if st.is_closed() {
                break;
            }

            handle_agent(st.clone(), session.clone(), task.name, task.query).await?;
        }

        // do query to fix errors:
        if let Some(last_line) = session.lock().await.last_line()
            && last_line.trim().starts_with("[Error]: ")
        {
            let query = str!(
                "Study the errors of the above execution, and if you can, then take action to correct mistakes else return empty tasks []."
            );

            Box::pin(delegate_tasks_cycled(
                st.clone(),
                session.clone(),
                query,
                recurs.clone(),
            ))
            .await;

            return Ok(());
        }
    }

    // print EOF:
    st.send(Ok(Bytes::from(fmt!(
        "\n\n[Duration]: {} ms\n[EOF]",
        session.lock().await.exec_time()
    ))))
    .ok();

    Ok(())
}

/// Handles agent query
async fn handle_agent(
    st: Stream,
    session: Arc<Mutex<SessionLogger>>,
    agent: String,
    query: String,
) -> Result<()> {
    // handle user/ai query:
    let actions = handle_query(st.clone(), session.clone(), &agent, &query).await?;

    // check client connection:
    if st.is_closed() {
        return Err(Error::ClientDisconnected.into());
    }

    // handle action call:
    for AgentAction { name, data } in actions {
        handle_action(st.clone(), &agent, &name, data).await?;
    }

    Ok(())
}

/// Handles query by LM
async fn handle_query(
    tx: Stream,
    session: Arc<Mutex<SessionLogger>>,
    agent: &str,
    query: &str,
) -> Result<Vec<AgentAction>> {
    // log info:
    {
        info!("⏳ Processing query: {:.100}", query.replace('\n', "\\n"));
        tx.send(Ok(Bytes::from(fmt!("[Processing]: {query}\n"))))
            .ok();
    }

    // read prompt:
    let prompt = utils::read_prompt("handle-query")
        .await?
        .replace("{EXAMPLES}", &Agents::exmpls(agent).await.join("\n"));

    // create query:
    let history = session.lock().await.results().clone();
    let mut request = utils::completions()?
        .system_message(vec![
            utils::read_prompt("assistant-character").await?.into(),
        ])
        .assistant_message(history.into_iter().map(|item| item.into()).collect())
        .system_message(vec![prompt.into()])
        .user_message(vec![query.into()])
        .tools(Agents::tools(agent).await);

    // read response:
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
async fn handle_action(tx: Stream, agent: &str, action: &str, data: JsonValue) -> Result<()> {
    // logging action start:
    {
        let data_str = json::to_string(&data).unwrap();
        info!("⏳ Handling {agent} action: {action} {data_str}",);
        tx.send(Ok(Bytes::from(fmt!("[Handling]: {action} {data_str}\n",))))
            .ok();
    }

    // create request to agent:
    let port = Settings::get().server.port;
    let client = Client::new();
    let response = client
        .post(fmt!("http://127.0.0.1:{port}/call/{agent}/{action}"))
        .json(&data)
        .send()
        .await?;

    // read agent response stream:
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        let _ = tx.send(Ok(bytes)).ok();
    }
    tx.send(Ok(Bytes::from("\n"))).ok();

    Ok(())
}
