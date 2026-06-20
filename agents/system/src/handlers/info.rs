use crate::prelude::*;

/// API: Handles the `/info` action
pub async fn handle_info() -> Response {
    Response::ok().json(&Settings::get().agent)
}
