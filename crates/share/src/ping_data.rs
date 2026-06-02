use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The ping response data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PingData {
    #[serde(default)]
    pub log_file: Option<PathBuf>,
    #[serde(default)]
    pub config_file: Option<PathBuf>,
}
