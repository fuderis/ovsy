use crate::{PowerAction, prelude::*};

/// API: Handles the `/tool/status` action
pub async fn handle() -> Response {
    let body = Stream::body(async move |tx| {
        let msg = if let Some(action) = PowerAction::get().await.as_ref() {
            str!(
                "Power action {} is scheduled. Remaining time: {}s",
                action.mode,
                action.timeout
            )
        } else {
            str!("No power actions scheduled.")
        };

        tx.send(Chunk::answer(msg)).ok();
    });

    Response::ok().stream(body)
}
