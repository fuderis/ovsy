use crate::{Manager, prelude::*};
use ovsy_shared::StatusResponse;

/// API: Refresh the server settings & agents list
pub async fn handle() -> Response {
    // update settings:
    if let Err(e) = Settings::update().await {
        return Response::ok().json(&StatusResponse::Error { error: str!("{e}") });
    }

    // update agents:
    if let Err(e) = Manager::update().await {
        return Response::ok().json(&StatusResponse::Error { error: str!("{e}") });
    }

    let agents = Manager::agents_list().await;
    Response::ok().json(&StatusResponse::Success { agents })
}
