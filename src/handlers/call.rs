use crate::prelude::*;
use reqwest::Client;
use tokio::{io::BufReader, process::Command};

/// Api '/call/:tool/:action' handler
pub async fn handle(
    Paths((name, action)): Paths<(String, String)>,
    Json(data): Json<JsonValue>,
) -> impl IntoResponse {
    let body = Stream::spawn(
        move |st| async move {
            handle_tool(st, name, action, data).await;
        },
        move |msg| async move {
            match msg {
                Ok(bytes) => Ok(bytes),
                Err(e) => {
                    error!("{e}");
                    Ok(Bytes::from(fmt!("[Error]: {e}")))
                }
            }
        },
    )
    .await;

    (
        StatusCode::OK,
        HeaderMap::from_iter(map! {
            header::CONTENT_TYPE =>
            "application/octet-stream".parse().unwrap(),
        }),
        Body::from_stream(body),
    )
        .into_response()
}

/// Handles user tool
pub async fn handle_tool(st: Stream, name: String, action: String, data: JsonValue) {
    info!(
        "Call tool '{name}/{action}', POST data: {}",
        json::to_string(&data).unwrap_or_default()
    );

    // search tool by name:
    let tool = match Tools::get(&name).await {
        Some(t) => t,
        _ => {
            st.send(Err(Error::UnexpectToolName(name.clone()).into()))
                .ok();
            return;
        }
    };

    // do server query:
    if let Some(server) = &tool.manifest.server {
        let port = server.port;
        let response = match Client::new()
            .post(fmt!("http://127.0.0.1:{port}/{action}"))
            .json(&data)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                st.send(Err(e.into())).ok();
                return;
            }
        };

        // streaming response:
        let mut response_stream = response.bytes_stream();
        while let Some(chunk) = response_stream.next().await {
            if let Ok(bytes) = chunk {
                st.send(Ok(bytes)).ok();
            }
        }
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
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                st.send(Err(e.into())).ok();
                return;
            }
        };

        // streaming response:
        let stdout = child.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout);
        let mut buf = vec![0u8; 1024];

        loop {
            match reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let bytes = Bytes::copy_from_slice(&buf[..n]);
                    st.send(Ok(bytes)).ok();
                }
                Err(_) => break,
            }
        }
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
