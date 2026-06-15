use crate::{Manager, prelude::*};
use ovsy_share::StatusData;

/// API: Refresh the server settings & agents list
pub async fn status() -> Response {
    let agents = Manager::agents_list().await;
    Response::ok().json(&StatusData::Success { agents })
}
