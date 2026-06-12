use crate::prelude::*;
use anylm::Message;
use ovsy_share::{MessagesQuery, SessionID};

/// API: Handles the session messages retrieval
pub async fn handle(data: Json<MessagesQuery>) -> Response {
    let session_id = data.0.session_id;

    match handle_messages(session_id).await {
        Ok(messages) => Response::ok().json(&messages),
        Err(e) => {
            error!("[handle_messages{{sid={session_id}}}] {e}");
            Response::bad_request().text(e.to_string())
        }
    }
}

/// Retrieves the session messages and initializes the session if it doesn't exist
#[log(skip_all, fields(sid = %session_id))]
async fn handle_messages(session_id: SessionID) -> Result<Vec<Message>> {
    // read session messages from db:
    let session = Session::new(session_id).await?;
    let db_messages = session.read_messages().await?;

    Ok(db_messages)
}
