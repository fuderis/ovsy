use crate::{power::DEFERRED_POWER_ACTION, prelude::*};

/// The request POST data
#[derive(Debug, Deserialize)]
pub struct QueryData {}

/// Api '/power/cancel' handler
pub async fn handle(Json(_data): Json<QueryData>) -> impl IntoResponse {
    // creating HTTP stream body:
    let body = Stream::body(move |tx| async move {
        let mut session = Session::new(tx);

        session
            .think("Checking for active power operations...")
            .await
            .ok();

        // we are trying to remove the task from the global state:
        let msg = if let Some(mode) = DEFERRED_POWER_ACTION.lock().await.take() {
            session
                .think(str!("Canceling deferred power action '{mode}'..."))
                .await
                .ok();

            sleep(Duration::from_secs(2)).await;
            str!("Power action {mode} is canceled!")
        } else {
            str!("Nothing to cancel.")
        };

        // sending the final response in session style:
        session.info(msg).await.ok();
    });

    // send stream to client:
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/octet-stream")],
        Body::from_stream(body),
    )
        .into_response()
}
