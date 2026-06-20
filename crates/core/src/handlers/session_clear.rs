use crate::prelude::*;
use ovsy_share::SessionID;

/// API: Handles the session clear method
#[log(skip_all, fields(sid = %sid.0))]
pub async fn session_clear(sid: Paths<SessionID>) -> Response {
    let session_id = sid.0;

    match handle_clear(session_id).await {
        Ok(_) => Response::ok(),
        Err(e) => {
            error!("{e}");
            Response::bad_request().text(e.to_string())
        }
    }
}

/// Completely clears the session message history
pub async fn handle_clear(session_id: SessionID) -> Result<()> {
    info!("Clearing history for session: {session_id}");

    // clear session messages:
    let session = Session::new(session_id).await?;
    session.clear().await?;

    Ok(())
}
