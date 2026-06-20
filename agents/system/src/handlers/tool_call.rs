use crate::{prelude::*, tools};

/// API: Handles the `/tools/call/{tool}` action
#[log(skip_all, fields(tool = %name.0))]
pub async fn handle_tool_call(name: Paths<String>, data: Json<JsonValue>) -> Response {
    info!("Initialized the `{}` tool handling", name.0);

    let body = Stream::body(async move |tx| {
        if let Err(e) = call_tool(tx.clone(), name.0, data.0).await {
            error!("{e}");
            tx.send(Chunk::error(e.to_string())).ok();
        }
    });

    Response::ok().stream(body)
}

/// Calls the agent tool
async fn call_tool(tx: Arc<StreamSender<Bytes>>, name: String, data: JsonValue) -> Result<()> {
    match name.as_ref() {
        "volume" => tools::handle_volume(tx.clone(), json::from_value(data)?).await,
        "power" => tools::handle_power(tx.clone(), json::from_value(data)?).await,
        "theme" => tools::handle_theme(tx.clone(), json::from_value(data)?).await,
        _ => Err(Error::UnknownTool(name).into()),
    }
}
