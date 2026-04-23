use crate::prelude::*;
use ovsy_shared::RefreshResponse;

/// API: Refresh the server settings & agents list
pub async fn refresh_handler() -> Response {
    if let Err(e) = Settings::update().await {
        Response::ok().json(&RefreshResponse::Error { error: str!("{e}") })
    } else {
        Response::ok().json(&RefreshResponse::Success { agents: vec![] })
    }
}
