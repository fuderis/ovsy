use crate::prelude::*;
use reqwest::Client;
use tokio::process::Command;

/// Api '/tool/{name}/{action}' handler
pub async fn handle(
    Paths((name, action)): Paths<(String, String)>,
    Json(data): Json<JsonValue>,
) -> Json<JsonValue> {
    match handle_tool(name, action, data).await {
        Ok(_) => Json(json!({ "status": 200 })),
        Err(e) => {
            err!("{e}");
            Json(json!({ "status": 500, "error": fmt!("{e}") }))
        }
    }
}

/// Handles user tool
pub async fn handle_tool(name: String, action: String, data: JsonValue) -> Result<()> {
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
        let url = fmt!("http://127.0.0.1:{port}/{action}");

        // send request to server:
        let client = Client::new();
        let response = client.post(url).json(&data).send().await?;

        // check status:
        if response.status() != 200 {
            return Err(
                Error::ToolBadStatus(response.status().as_u16(), response.text().await?).into(),
            );
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
                // key=value или просто value.to_string()
                let arg = fmt!("{}={}", key, to_cmd_arg(&value));
                cmd.arg(&arg);
            }
        }

        // run command:
        let output = cmd.output().await?;
        if !output.status.success() {
            return Err(Error::ToolExecFailed(
                tool.manifest.tool.name.clone(),
                str!(String::from_utf8_lossy(&output.stderr)),
            )
            .into());
        }
    }

    Ok(())
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
