use crate::{power::DEFERRED_POWER_ACTION, prelude::*};

/// The request POST data
#[derive(Deserialize)]
pub struct QueryData {}

/// Api '/power' handler
pub async fn handle(Json(_data): Json<QueryData>) -> impl IntoResponse {
    let msg = if let Some(mode) = DEFERRED_POWER_ACTION.lock().await.take() {
        sleep(Duration::from_secs(2)).await;
        fmt!("[Success] Power action {mode} is canceled!")
    } else {
        fmt!("[Success] Nothing to cancel.")
    };

    (
        StatusCode::OK,
        HeaderMap::from_iter(map! {
            header::CONTENT_TYPE => "text/plain".parse().unwrap()
        }),
        Body::new(msg),
    )
        .into_response()
}
