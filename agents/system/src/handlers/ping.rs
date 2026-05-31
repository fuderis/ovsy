use crate::prelude::*;
use ovsy_shared::PingData;

/// API: Handles the `/ping` action
pub async fn handle() -> Response {
    Response::ok().json(&PingData {
        log_file: Logger::path(),
        config_file: None,
    })
}
