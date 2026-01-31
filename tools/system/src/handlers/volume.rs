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
pub async fn handle(Json(data): Json<QueryData>) -> Json<JsonValue> {
    let vol = if !data.force {
        Audio::get_volume() as i32 + data.delta
    } else {
        data.delta
    }
    .clamp(0, 100) as u8;

    info!("Set volume to '{vol}%'");
    match Audio::set_volume(vol) {
        Ok(_) => Json(json!({ "status": 200 })),
        Err(e) => {
            err!("Failed to set volume: {e}");
            Json(json!({ "status": 500, "error": fmt!("{e}") }))
        }
    }
}
