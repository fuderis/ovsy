use crate::prelude::*;
use anylm::{AiChunk, Completions};
use ovsy_shared::{Chunk, UserQuery};

/// API: The user query handler
pub async fn query_handler(data: Json<UserQuery>) -> Response {
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
        .system_message(vec![ai_conf.assist_prompt.into()])
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
