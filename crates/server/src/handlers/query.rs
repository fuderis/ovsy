use crate::prelude::*;
use anylm::{AiChunk, Completions};
use ovsy_shared::{Chunk, UserQuery};

/// API: The user query handler
pub async fn handle(data: Json<UserQuery>) -> Response {
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

    // prepare prompt:
    let dt = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let prompt = ai_conf
        .assist_prompt
        .replace("{AGENTS_LIST}", &Manager::agents_list_doc().await)
        .replace("{DATETIME_UTC}", &dt);

    // create request to ai:
    let mut request = Completions::try_from(ai_conf.completions)?
        .system_message(vec![prompt.into()])
        .messages(data.messages)
        .tool(Manager::task_tool().await);

    // send request:
    let mut response = request.send().await?;

    while let Some(chunk) = response.next().await {
        match chunk? {
            AiChunk::Text { text } => {
                tx.send(Chunk::answer_bytes(text))?;
            }
            AiChunk::Tool { name, json_str } => {
                // TODO: ...
                tx.send(Chunk::answer_bytes(str!(
                    "**Tool call:** {name}({json_str})"
                )))?;
            }
        }
    }

    Ok(())
}
