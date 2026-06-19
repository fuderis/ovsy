use crate::{prelude::*, tools};

/// API: Handles the `/call/{tool}` action
pub async fn handle(name: Paths<String>, data: Json<JsonValue>) -> Response {
    let body = Stream::body(async move |tx| {
        if let Err(e) = call_tool(tx.clone(), name.0, data.0).await {
            tx.send(Chunk::error(e.to_string())).ok();
        }
    });

    Response::ok().stream(body)
}

/// Calls the agent tool
async fn call_tool(tx: Arc<StreamSender<Bytes>>, name: String, data: JsonValue) -> Result<()> {
    match name.as_ref() {
        "volume" => tools::handle_volume(tx.clone(), data).await,
        "power" => tools::handle_power(tx.clone(), data).await,
        _ => Err(Error::UnknownTool(name).into()),
    }
}
