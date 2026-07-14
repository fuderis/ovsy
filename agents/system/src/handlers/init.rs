use crate::prelude::*;

/// API: Handles the server ping
pub async fn handle_ping() -> Response {
    Response::ok().text("pong")
}

/// API: Handles the agent initialization
pub async fn handle_init() -> Response {
    Response::ok().json(&Settings::get().metadata)
}

/// API: Handles the agent skills receiving
pub async fn handle_skills_list() -> Response {
    Response::ok().json(&Settings::get().metadata.skills)
}
