use crate::prelude::*;
use anylm::Message;
use ovsy_share::SessionID;

/// API: Handles the session messages retrieval
#[log(skip_all, fields(sid = %sid.0))]
pub async fn sessions_get(sid: Paths<SessionID>) -> Response {
    let session_id = sid.0;

    match get_or_init(session_id).await {
        Ok(messages) => Response::ok().json(&messages),
        Err(e) => {
            error!("{e}");
            Response::bad_request().text(e.to_string())
        }
    }
}

/// Retrieves the session messages and initializes the session if it doesn't exist
async fn get_or_init(session_id: SessionID) -> Result<Vec<Message>> {
    // read session messages from db:
    let session = Session::new(session_id).await?;
    let db_messages = session.read_messages().await?;

    Ok(db_messages)
}
