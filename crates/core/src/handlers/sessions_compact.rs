use crate::prelude::*;
use anylm::{AiChunk, Completions, Message, Messages};
use ovsy_share::{Chunk, CompactQuery, SessionID};

/// API: Handles the session compression
#[log(skip_all, fields(sid = %sid.0))]
pub async fn sessions_compact(sid: Paths<SessionID>, data: Json<CompactQuery>) -> Response {
    let session_id = sid.0;
    let CompactQuery { preserve } = data.0;

    let current = Span::current();
    let body = Stream::body(move |tx| {
        async move {
            if let Err(e) = handle_compact(
                session_id,
                tx.clone(),
                preserve.unwrap_or_else(|| Settings::get().assistant.preserve_messages),
            )
            .await
            {
                error!("{e}");
                tx.send(Chunk::error(str!(e))).ok();
            }
        }
        .instrument(current)
    });

    Response::ok().stream(body)
}

/// Compresses the session messages
async fn handle_compact(session_id: SessionID, tx: Sender, preserve: usize) -> Result<()> {
    info!("Compressing session messages (preserve: {preserve})");

    let ai_conf = Settings::get().assistant.clone();
    let session = Session::new(session_id).await?;

    let db_messages = session.read_messages().await?;
    info!("{db_messages:#?}");
    let mut messages = Messages::from(db_messages);

    // preserve messages:
    let to_preserve = messages.slice(-(preserve as isize));

    if messages.messages.is_empty() {
        warn!("Nothing to compress, skip");
        tx.send(Chunk::finish()).ok();
        return Ok(());
    }

    let compress_count = messages.messages.len();
    let messages = messages.user(vec![ai_conf.compress_prompt.into()]).wrap();

    let mut response = Completions::try_from(ai_conf.compression)?
        .send(messages)
        .await?;

    let mut full_compressed_text = String::new();

    while let Some(chunk) = response.next().await {
        if let AiChunk::Text(text_part) = chunk? {
            tx.send(Chunk::answer(text_part.clone()))?;
            full_compressed_text.push_str(&text_part);
        }
    }

    // save to database:
    let compressed_message = Message::assistant(vec![full_compressed_text.into()], vec![]);
    session
        .insert_and_shift(compressed_message, to_preserve, compress_count)
        .await?;

    // finish work:
    tx.send(Chunk::finish()).ok();

    info!("Compression finished");
    Ok(())
}
