use crate::prelude::*;
use anylm::{AiChunk, Completions, Messages};
use ovsy_share::{Chunk, UserQuery};

/// API: Compressing the context history
pub async fn handle(data: Json<UserQuery>) -> Response {
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

    // create messages wrap:
    let messages = Messages::new()
        .messages(data.messages)
        .user(vec![ai_conf.compress_prompt.into()])
        .wrap();

    // send request to ai:
    let mut response = Completions::try_from(ai_conf.compression)?
        .send(messages)
        .await?;

    // read response chunks:
    while let Some(chunk) = response.next().await {
        if let AiChunk::Text(text_part) = chunk? {
            tx.send(Chunk::answer(text_part))?;
        }
    }

    Ok(())
}
