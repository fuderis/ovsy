use crate::{Manager, prelude::*};
use ovsy_share::StatusData;

/// API: Update the server settings & agents list
pub async fn update() -> Response {
    // update settings:
    if let Err(e) = Settings::update().await {
        return Response::ok().json(&StatusData::Error { error: str!("{e}") });
    }

    // update agents:
    if let Err(e) = Manager::update().await {
        return Response::ok().json(&StatusData::Error { error: str!("{e}") });
    }

    let agents = Manager::agents_list().await;
    Response::ok().json(&StatusData::Success { agents })
}
