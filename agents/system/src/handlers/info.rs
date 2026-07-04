use crate::prelude::*;

/// API: Returns the agent info
pub async fn handle_info() -> Response {
    Response::ok().json(&Settings::get().agent)
}
