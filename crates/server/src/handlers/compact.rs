use crate::prelude::*;
use anylm::{AiChunk, Completions, Messages};
use ovsy_shared::{Chunk, UserQuery};

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

    // prepare messages:
    let messages = Messages::new()
        .messages(data.messages)
        .user(vec![ai_conf.compress_prompt.into()])
        .wrap();

    // send request:
    let mut response = Completions::try_from(ai_conf.compression)?
        .send(messages)
        .await?;

    while let Some(chunk) = response.next().await {
        match chunk? {
            AiChunk::Text(text_part) => {
                tx.send(Chunk::answer(text_part))?;
            }
            _ => {}
        }
    }

    Ok(())
}
