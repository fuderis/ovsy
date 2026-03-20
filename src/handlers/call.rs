use crate::{SessionChunk, prelude::*};
use reqwest::Client;
use tokio::{io::AsyncReadExt, io::BufReader, process::Command};

/// Api '/call/:agent/:action' handler
pub async fn handle(
    Paths((name, action)): Paths<(String, String)>,
    Json(data): Json<JsonValue>,
) -> impl IntoResponse {
    let body = Stream::body(move |tx| async move {
        handle_action(tx, name, action, data).await;
    });

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/octet-stream")],
        Body::from_stream(body),
    )
        .into_response()
}

/// Handles an agent action
pub async fn handle_action(st: StreamSender<Bytes>, name: String, action: String, data: JsonValue) {
    // 1. Поиск инструмента
    let tool = match Agents::get(&name).await {
        Some(t) => t,
        _ => {
            send_error(
                &st,
                fmt!("Agent '{name}' not found"),
                Error::UnexpectedAgentName(name).to_string(),
            );
            return;
        }
    };

    // 2. Выполнение через HTTP (если указан порт)
    if let Some(port) = &tool.port {
        let response = match Client::new()
            .post(fmt!("http://127.0.0.1:{port}/{action}"))
            .json(&data)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                send_error(
                    &st,
                    fmt!("Failed to connect to agent {name}"),
                    e.to_string(),
                );
                return;
            }
        };

        let mut response_stream = response.bytes_stream();
        while let Some(chunk) = response_stream.next().await {
            if let Ok(bytes) = chunk {
                // Пересылаем сырые байты (которые уже должны быть SessionChunk в JSON)
                let _ = st.send(bytes);
            }
        }
    }
    // 3. Выполнение через бинарный файл (spawn)
    else {
        let exec_path = &tool.manifest.agent.exec;

        let mut cmd = Command::new(exec_path);
        cmd.stdout(std::process::Stdio::piped());
        cmd.kill_on_drop(true);

        if let JsonValue::Object(map) = data {
            for (key, value) in map {
                cmd.arg(fmt!("--{key}")).arg(to_cmd_arg(&value));
            }
        }

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                send_error(&st, fmt!("Failed to spawn agent {name}"), e.to_string());
                return;
            }
        };

        let stdout = child.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout);
        let mut buf = vec![0u8; 4096];

        loop {
            match reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let _ = st.send(Bytes::copy_from_slice(&buf[..n]));
                }
                Err(e) => {
                    send_error(&st, "Read error from agent process", e.to_string());
                    break;
                }
            }
        }
        let _ = child.wait().await;
    }
}

/// Помощник для отправки типизированной ошибки через байтовый стрим
fn send_error(st: &StreamSender<Bytes>, friendly_msg: impl Into<String>, technical_err: String) {
    let chunk = SessionChunk::Error {
        message: friendly_msg.into(),
        error: technical_err,
    };
    if let Ok(json) = json::to_vec(&chunk) {
        let _ = st.send(Bytes::from(json));
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
        JsonValue::Object(_) => json::to_string(value).unwrap_or_default(),
    }
}
