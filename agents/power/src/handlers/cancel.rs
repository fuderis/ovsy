use crate::{ACTIVE_ACTION, prelude::*};

/// API: Handles the `/tool/cancel` action
pub async fn handle() -> Response {
    let body = Stream::body(async move |tx| {
        let msg = if let Some((_, mode)) = ACTIVE_ACTION.lock().await.take() {
            str!("Power action {mode} is canceled!")
        } else {
            str!("Nothing to cancel.")
        };

        tx.send(Chunk::answer(msg)).ok();
    });

    Response::ok().stream(body)
}
