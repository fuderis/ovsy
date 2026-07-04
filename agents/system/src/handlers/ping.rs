use crate::prelude::*;
use ovsy_share::PingData;

/// API: Handles the server ping
pub async fn handle_ping() -> Response {
    Response::ok().json(&PingData {
        log_file: Logger::path(),
        config_file: None,
    })
}
