use crate::prelude::*;
use anylm::{AiChunk, Completions};
use ovsy_shared::{Chunk, UserQuery};

/// API: Compressing the context history
pub async fn compress_handler(data: Json<UserQuery>) -> Response {
    let body = Stream::body(move |tx| async move {
        if let Err(e) = handle_compression(tx.clone(), data.0).await {
            tx.send(Chunk::error(str!("{e}"))).ok();
        }
    });

    Response::ok().stream(body)
}

/// Context compression logic
async fn handle_compression(tx: Sender, data: UserQuery) -> Result<()> {
    let ai_conf = Settings::get().assistant.clone();

    // create request to ai:
    let mut request = Completions::try_from(ai_conf.compression)?
        .messages(data.messages)
        .user_message(vec![ai_conf.compress_prompt.into()]);

    // send request:
    let mut response = request.send().await?;

    while let Some(chunk) = response.next().await {
        match chunk? {
            AiChunk::Text { text } => {
                tx.send(Chunk::answer(text))?;
            }
            _ => {}
        }
    }

    Ok(())
}
