use anylm::{AiChunk, Completions};
use ovsy_server::prelude::*;
use ovsy_shared::{Chunk, RefreshResponse, UserQuery};

#[tokio::main]
async fn main() -> Result<()> {
    // init settings & logger:
    Settings::init(app_data().join("settings.toml")).await?;
    Logger::init(app_data().join("logs"), Settings::get().server.log_files).await?;

    // start server:
    Server::new()
        .post("/handle", query_handler)
        .post("/refresh", refresh_handler)
        .run(([127, 0, 0, 1], 7878))
        .await?;

    Ok(())
}

/// API: Refresh the server settings & agents list
async fn refresh_handler() -> Response {
    if let Err(e) = Settings::update().await {
        Response::ok().json(&RefreshResponse::Error { error: str!("{e}") })
    } else {
        Response::ok().json(&RefreshResponse::Success { agents: vec![] })
    }
}

/// API: The user query handler
async fn query_handler(data: Json<UserQuery>) -> Response {
    let body = Stream::body(move |tx| async move {
        if let Err(e) = handle_query(tx.clone(), data.0).await {
            tx.send(Chunk::error(str!("{e}"))).ok();
        }
    });

    Response::ok().stream(body)
}

/// Handles the user query
async fn handle_query(tx: Sender, data: UserQuery) -> Result<()> {
    let ai_conf = Settings::get().assistant.clone();

    // create request to ai:
    let mut request = Completions::try_from(ai_conf.completions)?
        .system_message(vec![ai_conf.prompt.into()])
        .messages(data.messages);

    // send request:
    let mut response = request.send().await?;

    while let Some(chunk) = response.next().await {
        match chunk? {
            AiChunk::Text { text } => {
                tx.send(Chunk::answer(text))?;
            }
            AiChunk::Tool { name, json_str } => {
                // TODO: ...
                dbg!(name, json_str);
            }
        }
    }

    Ok(())
}
