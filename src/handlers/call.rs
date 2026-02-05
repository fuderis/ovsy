use crate::prelude::*;
use reqwest::Client;
use tokio::process::Command;

/// Api '/call/:tool/:action' handler
pub async fn handle(
    Paths((name, action)): Paths<(String, String)>,
    Json(data): Json<JsonValue>,
) -> impl IntoResponse {
    match handle_tool(name, action, data).await {
        Ok(rx) => {
            let stream = stream::unfold(rx, |mut rx| async move {
                rx.recv()
                    .await
                    .map(|bytes| (Ok::<_, std::convert::Infallible>(bytes), rx))
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

/// Handles user tool
pub async fn handle_tool(
    name: String,
    action: String,
    data: JsonValue,
) -> Result<UnboundedReceiver<Bytes>> {
    info!(
        "Call tool '{name}/{action}', POST data: {}",
        json::to_string(&data).unwrap_or_default()
    );

    // search tool by name:
    let tool = Tools::get(&name)
        .await
        .ok_or(Error::UnexpectToolName(name.clone()))?;

    // do server query:
    if let Some(server) = &tool.manifest.server {
        let port = server.port;
        let response = Client::new()
            .post(fmt!("http://127.0.0.1:{port}/{action}"))
            .json(&data)
            .send()
            .await?;

        // streaming response:
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            while let Some(chunk) = stream.next().await {
                if let Ok(bytes) = chunk {
                    tx.send(bytes).ok();
                }
            }
        });

        Ok(rx)
    }
    // do exec run:
    else {
        let exec_path = &tool.manifest.tool.exec;
        let mut cmd = Command::new(exec_path);
        cmd.kill_on_drop(true);

        // add command args:
        if let JsonValue::Object(map) = data {
            for (key, value) in map {
                // key=value or just value.to_string()
                let arg = fmt!("{}={}", key, to_cmd_arg(&value));
                cmd.arg(&arg);
            }
        }

        // run command:
        let mut child = cmd.spawn()?;

        // streaming response:
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let stdout = child.stdout.take().unwrap();
            let mut reader = tokio::io::BufReader::new(stdout);
            let mut buf = vec![0u8; 1024];

            loop {
                match reader.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        let bytes = Bytes::copy_from_slice(&buf[..n]); // Bytes!
                        tx.send(bytes).ok();
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(rx)
    }
}

/// Converts json value into command argument
fn to_cmd_arg(value: &JsonValue) -> String {
    match value {
        JsonValue::Null => "null".to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::String(s) => s.clone(),
        JsonValue::Array(arr) => arr.iter().map(to_cmd_arg).collect::<Vec<_>>().join(","),
        JsonValue::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}
