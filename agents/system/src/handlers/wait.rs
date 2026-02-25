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
    let total_secs = data.millis.unwrap_or(0) / 1000
        + data.secs.unwrap_or(0)
        + data.mins.unwrap_or(0) * 60
        + data.hours.unwrap_or(0) * 60 * 60;

    if total_secs == 0 {
        return (StatusCode::BAD_REQUEST, "No time specified").into_response();
    }

    // create receiver:
    let (tx, rx) = mpsc::channel::<Bytes>(32);

    // send message:
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        // display time:
        let mut parts: Vec<String> = Vec::new();

        if let Some(h) = data.hours {
            parts.push(format!("{} hour{}", h, if h == 1 { "" } else { "s" }));
        }
        if let Some(m) = data.mins {
            parts.push(format!("{} minute{}", m, if m == 1 { "" } else { "s" }));
        }
        if let Some(s) = data.secs {
            parts.push(format!("{} second{}", s, if s == 1 { "" } else { "s" }));
        }
        if let Some(ms) = data.millis {
            parts.push(format!("{} ms", ms));
        }

        tx_clone
            .send(Bytes::from(fmt!("Total wait time: {}\n", parts.join(", "))))
            .await
            .ok();
    });

    // send messages on last 10 seconds:
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        let remaining = if total_secs > 10 {
            sleep(Duration::from_secs(total_secs - 10)).await;
            10
        } else {
            total_secs
        };

        for r in (0..remaining).rev() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            if r == 0 {
                continue;
            }
            tx_clone
                .send(Bytes::from(fmt!("{} seconds remaining\n", r)))
                .await
                .ok();
        }

        tx_clone.send(Bytes::from("Wait completed.")).await.ok();
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
