use crate::prelude::*;
use ovsy_shared::HealthData;

/// API: Handles the `/health` action
pub async fn handle() -> Response {
    Response::ok().json(&HealthData {
        log_file: Logger::path(),
        config_file: None,
    })
}
