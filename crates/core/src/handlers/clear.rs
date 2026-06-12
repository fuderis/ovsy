use crate::prelude::*;
use ovsy_share::{Chunk, ClearQuery, SessionID};
use tokio::time;

/// API: Handles the session clear method
pub async fn handle(data: Json<ClearQuery>) -> Response {
    let session_id = data.0.session_id;

    let body = Stream::body(move |tx| async move {
        if let Err(e) = handle_clear(session_id).await {
            error!("[handle_clear{{sid={session_id}}}] {e}");
            tx.send(Chunk::error(str!(e))).ok();
        }
    });

    Response::ok().stream(body)
}

/// Completely clears the session message history
#[log(skip_all, fields(sid = %session_id))]
pub async fn handle_clear(session_id: SessionID) -> Result<()> {
    info!("Clearing history for session: {session_id}");

    // clear session messages:
    let session = Session::new(session_id).await?;
    session.clear().await?;

    // wait for unlock db:
    drop(session);
    time::sleep(Duration::from_millis(100)).await;

    Ok(())
}
