use crate::prelude::*;
use pc_remote::Audio;

/// The request POST data
#[derive(Deserialize)]
pub struct QueryData {
    delta: i32,
    #[serde(default)]
    force: bool,
}

/// Api '/volume' handler
pub async fn handle(Json(data): Json<QueryData>) -> impl IntoResponse {
    let vol = if !data.force {
        Audio::get_volume() as i32 + data.delta
    } else {
        data.delta
    }
    .clamp(0, 100) as u8;

    info!("Set volume to {vol}%");
    match Audio::set_volume(vol) {
        Ok(_) => {
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "text/plain".parse().unwrap());
            (
                StatusCode::OK,
                headers,
                Body::new(fmt!("Set volume to {vol}%")),
            )
                .into_response()
        }
        Err(e) => {
            error!("Failed to set volume: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}
