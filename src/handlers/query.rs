use crate::{LMKind, lms, prelude::*};
use reqwest::Client;
use tokio::fs;

/// The tool call data
type ToolCall = (String, HashMap<String, JsonValue>);

/// The request POST data
#[derive(Deserialize)]
pub struct QueryData {
    query: String,
}

/// Api '/query' handler
pub async fn handle(Json(data): Json<QueryData>) -> impl IntoResponse {
    match handle_query(data.query).await {
        Ok(calls) => {
            info!("Call tools order: {calls:?}");
            let (tx, rx) = mpsc::unbounded_channel();

            tokio::spawn(async move {
                for call in calls {
                    if let Err(e) = handle_tool(call, &tx).await {
                        tx.send(Bytes::from(fmt!("\nError: {e}").as_bytes().to_vec()))
                            .ok();
                        break;
                    }
                }
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
            error!("{e}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

/// Handles user query
async fn handle_query(query: String) -> Result<Vec<ToolCall>> {
    let cfg = Settings::read()?;
    info!("⏳ Handle query '{:.100}'..", query.replace("\n", "\\n"));

    // read prompt:
    let mut prompt_dir = path!("$/prompt");
    if !prompt_dir.exists() {
        prompt_dir = path!("$/../../prompt");
    }
    let prompt = fs::read_to_string(prompt_dir.join("handle-query.md")).await?;
    let prompt = prompt.replace("{DOCS}", &Tools::docs().await.join("\n\n"));

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
async fn handle_tool(call: ToolCall, tx: &UnboundedSender<Bytes>) -> Result<()> {
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
    while let Some(chunk) = stream.next().await {
        if let Ok(bytes) = chunk {
            let _ = tx.send(bytes);
        }
    }

    Ok(())
}
