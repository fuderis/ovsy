use crate::{Manager, prelude::*};
use ovsy_shared::StatusResponse;

/// API: Refresh the server settings & agents list
pub async fn handle() -> Response {
    let agents = Manager::agents_list().await;
    Response::ok().json(&StatusResponse::Success { agents })
}
