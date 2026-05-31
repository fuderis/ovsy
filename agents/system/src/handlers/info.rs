use crate::prelude::*;

/// API: Handles the `/info` action
pub async fn handle() -> Response {
    Response::ok().json(&Settings::get().agent)
}
