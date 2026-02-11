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

    let (tx, rx) = mpsc::unbounded_channel::<StdResult<Bytes, Bytes>>();

    // create stream body:
    let body = stream::unfold(rx, {
        let session = session.clone();
        move |mut rx| {
            let session = session.clone();
            async move {
                let msg = rx.recv().await?;
                match msg {
                    Ok(bytes) => {
                        // write output:
                        let mut guard = session.lock().await;
                        guard.write(&String::from_utf8_lossy(&bytes)).await.ok();

                        Some((Ok::<_, Infallible>(bytes), rx))
                    }
                    Err(bytes) => {
                        let bytes_str = String::from_utf8_lossy(&bytes);
                        error!("{bytes_str}");

                        // write error:
                        let mut guard = session.lock().await;
                        guard.write(&fmt!("[Error]: {bytes_str}")).await.ok();

                        // write EOF:
                        let dur_ms = guard.exec_time();
                        guard
                            .write(&fmt!("\n[Duration]: {dur_ms} ms\n[EOF]",))
                            .await
                            .ok();

                        Some((Ok::<_, Infallible>(bytes), rx))
                    }
                }
            }
        }
    });

    // spawn stream handler:
    tokio::spawn(stream(Arc::new(tx), session.clone(), data.query, 0));

    (
        StatusCode::OK,
        HeaderMap::from_iter(map! {
            header::CONTENT_TYPE => "application/octet-stream".parse().unwrap()
        }),
        Body::from_stream(body),
    )
        .into_response()
}

/// Handles stream (recursive)
async fn stream(
    tx: Arc<UnboundedSender<StdResult<Bytes, Bytes>>>,
    session: Arc<Mutex<SessionLogger>>,
    query: String,
    mut recurs: usize,
) {
    recurs += 1;
    if recurs > Settings::get().tools.recurs_limit {
        let _ = tx
            .send(Err(Bytes::from(Error::RecursionLimit.to_string())))
            .ok();
        return;
    } else if tx.is_closed() {
        let _ = tx
            .send(Err(Bytes::from(Error::ClientDisconnected.to_string())))
            .ok();
        return;
    }

    // handle user/ai query:
    let response = match handle_query(tx.clone(), session.clone(), &query).await {
        Ok(res) => {
            // check client status:
            if tx.is_closed() {
                let _ = tx
                    .send(Err(Bytes::from(Error::ClientDisconnected.to_string())))
                    .ok();
                return;
            }
            res
        }
        Err(e) => {
            let _ = tx.send(Err(Bytes::from(fmt!("LM error: {e}")))).ok();
            return;
        }
    };

    // handle tool call:
    if let Some(tool) = response.tool {
        if let Err(e) = handle_tool(tx.clone(), tool, response.data).await {
            let _ = tx.send(Err(Bytes::from(fmt!("Tool error: {}", e)))).ok();
            return;
        }
    } else {
        let content = response
            .data
            .get("content")
            .cloned()
            .unwrap_or_else(|| json!("No content"));
        let _ = tx.send(Ok(Bytes::from(fmt!("{}\n", content)))).ok();
    }

    // handle next query:
    if let Some(next_query) = response.query
        && !next_query.trim().is_empty()
    {
        // check client status:
        if tx.is_closed() {
            let _ = tx
                .send(Err(Bytes::from(Error::ClientDisconnected.to_string())))
                .ok();
            return;
        }

        tx.send(Ok(Bytes::from("\n"))).ok();
        Box::pin(stream(tx, session, next_query, recurs)).await;
    } else {
        tx.send(Ok(Bytes::from(fmt!(
            "\n[Duration]: {} ms",
            session.lock().await.exec_time()
        ))))
        .ok();
        tx.send(Ok(Bytes::from("\n[EOF]"))).ok();
    }
}

/// Handles user query
async fn handle_query(
    tx: Arc<UnboundedSender<StdResult<Bytes, Bytes>>>,
    session: Arc<Mutex<SessionLogger>>,
    query: &str,
) -> Result<LmResponse> {
    let past_results = session.lock().await.results().join("\n");
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
        .replace("{HISTORY}", &past_results)
        .replace("{DOCS}", &Tools::docs().await.join("\\n\\n"))
        .replace("{EXAMPLES}", &Tools::exmpls().await.join("\\n"));

    // DEBUG: past results
    dbg!(past_results);

    let json = match &cfg.lms.slm_kind {
        LMKind::LMStudio => {
            let small = Settings::get().lmstudio.small.clone();
            lms::lmstudio::handle_query(prompt, query, small).await?
        }
    };

    // parse response:
    let re = regex::Regex::new(r"^\s*```(?:\S+\b)?|\n```\s*$")?;
    let json = re.replace_all(&json, "").trim().to_string();

    // DEBUG: LM response
    dbg!(&json);

    Ok(serde_json::from_str(&json)?)
}

/// Handles tool call
async fn handle_tool(
    tx: Arc<mpsc::UnboundedSender<StdResult<Bytes, Bytes>>>,
    tool: String,
    data: HashMap<String, JsonValue>,
) -> Result<()> {
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
