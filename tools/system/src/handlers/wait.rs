use crate::prelude::*;

/// The request POST data
#[derive(Deserialize)]
pub struct QueryData {
    hours: Option<u64>,
    mins: Option<u64>,
    secs: Option<u64>,
    millis: Option<u64>,
}

/// Api '/wait' handler
pub async fn handle(Json(data): Json<QueryData>) -> impl IntoResponse {
    let total_millis = data.millis.unwrap_or(0)
        + data.secs.unwrap_or(0) * 1000
        + data.mins.unwrap_or(0) * 60 * 1000
        + data.hours.unwrap_or(0) * 60 * 60 * 1000;

    if total_millis == 0 {
        return (StatusCode::BAD_REQUEST, "No time specified").into_response();
    }

    let total_secs = total_millis / 1000;

    // create receiver:
    let (tx, rx) = mpsc::channel::<Bytes>(32);

    // send message:
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        let _ = tx_clone
            .send(Bytes::from(format!(
                "Total wait time: {} seconds\n",
                total_secs
            )))
            .await;
    });

    // send messages on last 10 seconds:
    let tx2 = tx.clone();
    tokio::spawn(async move {
        let remaining = if total_secs > 10 {
            tokio::time::sleep(Duration::from_secs(total_secs - 10)).await;
            let _ = tx2.send(Bytes::from("waiting...\n")).await;
            10
        } else {
            10 - total_secs
        };

        for r in (0..remaining).rev() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            if r == 0 {
                continue;
            }
            let _ = tx2
                .send(Bytes::from(format!("{} seconds remaining\n", r)))
                .await;
        }

        let _ = tx2.send(Bytes::from("wait completed\n")).await;
    });

    // create stream:
    let stream = stream::unfold(rx, |mut rx| async move {
        rx.recv()
            .await
            .map(|bytes| (Ok::<_, Infallible>(bytes), rx))
    });

    (
        StatusCode::OK,
        HeaderMap::from_iter(map! {
            header::CONTENT_TYPE => "text/event-stream".parse().unwrap(),
            header::CACHE_CONTROL => "no-cache".parse().unwrap(),
            header::CONNECTION => "keep-alive".parse().unwrap(),
        }),
        Body::from_stream(stream),
    )
        .into_response()
}
