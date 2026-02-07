use crate::{LMKind, SessionLogger, lms, prelude::*};
use reqwest::Client;
use tokio::fs;

/// The request POST data
#[derive(Deserialize)]
pub struct QueryData {
    query: String,
    session_id: String,
}

/// Api '/query' handler
pub async fn handle(Json(data): Json<QueryData>) -> impl IntoResponse {
    let session = Arc::new(Mutex::new(
        SessionLogger::new(data.session_id, &data.query)
            .await
            .map_err(|e| fmt!("Failed to create session: {e}"))
            .unwrap(),
    ));

    macro_rules! logerr {
        ($session: expr, $($args:tt)*) => {{
            let msg = fmt!($($args)*);
            $session.lock().await.write(&msg).await.ok();
            error!("{msg}");
            }};
    }

    match handle_query(data.query).await {
        Ok(calls) => {
            info!(
                "Call tools order: {}",
                json::to_string(&calls).unwrap().replace("\n", "\\n")
            );
            session.lock().await.write_tool_calls(&calls).await.ok();
            let (tx, rx) = mpsc::unbounded_channel();

            tokio::spawn(async move {
                for call in calls {
                    if tx.is_closed() {
                        logerr!(&session, "{}", Error::ClientDisconnected);
                        return;
                    }
                    if let Err(e) = handle_tool(session.clone(), call, &tx).await {
                        logerr!(&session, "{}", Error::ClientDisconnected);
                        tx.send(Bytes::from(fmt!("\nError: {e}").as_bytes().to_vec()))
                            .ok();
                        return;
                    }
                }

                session.lock().await.finish().await.ok();
            });

            let stream = stream::unfold(rx, |mut rx| async move {
                rx.recv()
                    .await
                    .map(|bytes| (Ok::<_, Infallible>(bytes), rx))
            });

            (
                StatusCode::OK,
                HeaderMap::from_iter(map!(
                    header::CONTENT_TYPE =>
                    "application/octet-stream".parse().unwrap(),
                )),
                Body::from_stream(stream),
            )
                .into_response()
        }
        Err(e) => {
            session.lock().await.write_tool_calls(&[]).await.ok();
            logerr!(session, "{e}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

/// Handles user query
async fn handle_query(query: String) -> Result<Vec<ToolCall>> {
    let cfg = Settings::read()?;
    info!("⏳ Handle query '{:.100}'..", query.replace("\n", "\\n"));

    // read prompt or create default:
    let prompt_dir = app_data().join("prompts");
    let prompt_file = prompt_dir.join("handle-query.md");

    if !prompt_file.exists() {
        fs::create_dir(prompt_dir).await?;
        fs::write(
            &prompt_file,
            fs::read(path!("$/../../default/prompts/handle-query.md")).await?,
        )
        .await?;
    }

    let prompt = fs::read_to_string(prompt_file)
        .await?
        .replace("{DOCS}", &Tools::docs().await.join("\n\n"))
        .replace("{EXMPLS}", &Tools::exmpls().await.join("\n"));

    // handle query by LLM:
    let json = match &cfg.lms.slm_kind {
        LMKind::LMStudio => {
            let small = Settings::get().lmstudio.small.clone();
            lms::lmstudio::handle_query(prompt, &query, small).await?
        }
    };

    // trim code block:
    let re = re!(r#"^\s*```(?:\S+\b)?|\n```\s*$"#);
    let json = re.replace_all(&json, "").trim().to_string();
    let calls: Vec<ToolCall> =
        json::from_str(&json).map_err(|e| fmt!("Invalid LM response format: {e}"))?;

    Ok(calls)
}

/// Handles tool call
async fn handle_tool(
    session: Arc<Mutex<SessionLogger>>,
    call: ToolCall,
    tx: &UnboundedSender<Bytes>,
) -> Result<()> {
    let (tool_name, tool_data) = call;

    // parse tool call:
    let mut spl = tool_name.splitn(2, "/");
    let name = spl
        .next()
        .ok_or(Error::InvalidToolNameFormat(tool_name.clone()))?
        .to_owned();
    let action = spl
        .next()
        .ok_or(Error::InvalidToolNameFormat(tool_name.clone()))?
        .to_owned();
    let data = json::to_value(&tool_data)?;

    // do tool call:
    let port = Settings::get().server.port;
    let response = Client::new()
        .post(fmt!("http://127.0.0.1:{port}/call/{name}/{action}"))
        .json(&data)
        .send()
        .await?;

    // streaming response:
    let mut stream = response.bytes_stream();
    loop {
        if tx.is_closed() {
            return Err(Error::ClientDisconnected.into());
        }

        if let Some(chunk) = stream.next().await {
            if let Ok(bytes) = chunk {
                session
                    .lock()
                    .await
                    .write(&String::from_utf8_lossy(&bytes))
                    .await
                    .ok();
                tx.send(bytes).ok();
            }
        } else {
            break;
        }
    }

    Ok(())
}
