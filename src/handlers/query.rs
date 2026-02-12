use crate::{LMKind, SessionLogger, lms, prelude::*};
use reqwest::Client;
use tokio::fs;

/// The request POST data
#[derive(Deserialize)]
pub struct QueryData {
    query: String,
    session_id: String,
}

/// The LM response structure
#[derive(Deserialize, Clone)]
pub struct LmResponse {
    tool: Option<String>,
    data: HashMap<String, JsonValue>,
    query: Option<String>,
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
    let session_clone = session.clone();

    // create stream body:
    let body = Stream::spawn(
        async move |tx| {
            handle_query_cycle(tx.clone(), session_clone, data.query, 0).await;
        },
        async move |msg| match msg {
            Ok(bytes) => {
                let mut guard = session.lock().await;
                guard.write(&String::from_utf8_lossy(&bytes)).await.ok();

                Ok(bytes)
            }
            Err(e) => {
                error!("{e}");

                let mut guard = session.lock().await;
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

/// Handles query cycle (recursive)
async fn handle_query_cycle(
    st: Stream,
    session: Arc<Mutex<SessionLogger>>,
    query: String,
    mut recurs: usize,
) {
    recurs += 1;
    let limit = Settings::get().tools.recurs_limit;
    if limit > 0 && recurs > limit {
        st.send(Err(Error::RecursionLimit.into())).ok();
        return;
    } else if st.is_closed() {
        st.send(Err(Error::ClientDisconnected.into())).ok();
        return;
    }

    // handle user/ai query:
    let response = match handle_query(st.clone(), session.clone(), &query).await {
        Ok(res) => {
            // check client status:
            if st.is_closed() {
                st.send(Err(Error::ClientDisconnected.into())).ok();
                return;
            }
            res
        }
        Err(e) => {
            st.send(Err(fmt!("LM error: {e}").into())).ok();
            return;
        }
    };

    // handle tool call:
    if let Some(tool) = response.tool {
        if let Err(e) = handle_tool(st.clone(), tool, response.data).await {
            st.send(Err(fmt!("Tool error: {}", e).into())).ok();
            return;
        }
    } else {
        let content = response
            .data
            .get("content")
            .cloned()
            .unwrap_or_else(|| json!("No content"));
        st.send(Ok(Bytes::from(fmt!("{}\n", content)))).ok();
    }

    // handle next query cycle:
    if let Some(next_query) = response.query
        && !next_query.trim().is_empty()
    {
        // check client status:
        if st.is_closed() {
            st.send(Err(Error::ClientDisconnected.into())).ok();
            return;
        }

        st.send(Ok(Bytes::from("\n"))).ok();
        Box::pin(handle_query_cycle(st, session, next_query, recurs)).await;
    } else {
        st.send(Ok(Bytes::from(fmt!(
            "\n[Duration]: {} ms\n[EOF]",
            session.lock().await.exec_time()
        ))))
        .ok();
    }
}

/// Handles query by LM
async fn handle_query(
    tx: Stream,
    session: Arc<Mutex<SessionLogger>>,
    query: &str,
) -> Result<LmResponse> {
    let results = session.lock().await.results().clone();
    let history = utils::cut_context_lines(&results[..], Settings::get().context.tokens_limit);

    // log info:
    let cfg = Settings::read()?;
    {
        info!("⏳ Processing query: {:.100}", query.replace('\n', "\\n"));
        tx.send(Ok(Bytes::from(fmt!("[Processing]: {query}\n"))))
            .ok();
    }

    let prompt_dir = app_data().join("prompts");
    let prompt_file = prompt_dir.join("handle-query.md");

    if !prompt_file.exists() {
        fs::create_dir_all(&prompt_dir).await?;
        fs::write(
            &prompt_file,
            fs::read(path!("$/../../default/prompts/handle-query.md")).await?,
        )
        .await?;
    }

    let prompt = fs::read_to_string(&prompt_file)
        .await?
        .replace("{HISTORY}", &history)
        .replace("{DOCS}", &Tools::docs().await.join("\\n\\n"))
        .replace("{EXAMPLES}", &Tools::exmpls().await.join("\\n"));

    // DEBUG: past results
    dbg!(history);

    let json = match &cfg.lms.slm_kind {
        LMKind::LMStudio => {
            let small = Settings::get().lmstudio.small_model.clone();
            lms::lmstudio::handle_query(prompt, query, small).await?
        }
    };

    // parse response:
    let re = re!(r"^\s*```(?:\S+\b)?|\n```\s*$");
    let json = re.replace_all(&json, "").trim().to_string();

    // DEBUG: LM response
    dbg!(&json);

    Ok(serde_json::from_str(&json)?)
}

/// Handles tool call
async fn handle_tool(tx: Stream, tool: String, data: HashMap<String, JsonValue>) -> Result<()> {
    {
        let data_str = json::to_string(&data).unwrap();
        info!("⏳ Handling tool call: {tool} {data_str}",);
        tx.send(Ok(Bytes::from(fmt!("[Handling]: {tool} {data_str}\n",))))
            .ok();
    }

    // parse tool name/action:
    let mut spl = tool.splitn(2, '/');
    let tool_name = spl
        .next()
        .ok_or(Error::InvalidToolNameFormat(tool.clone()))?
        .to_string();
    let tool_action = spl
        .next()
        .ok_or(Error::InvalidToolNameFormat(tool.clone()))?
        .to_string();

    // parse tool data:
    let tool_data = serde_json::to_value(data)?;

    let port = Settings::get().server.port;
    let client = Client::new();
    let response = client
        .post(fmt!(
            "http://127.0.0.1:{port}/call/{tool_name}/{tool_action}"
        ))
        .json(&tool_data)
        .send()
        .await?;

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        let _ = tx.send(Ok(bytes)).ok();
    }

    tx.send(Ok(Bytes::from("\n"))).ok();

    Ok(())
}
