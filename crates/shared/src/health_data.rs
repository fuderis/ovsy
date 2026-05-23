use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The health response structure
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthData {
    #[serde(default)]
    pub log_file: Option<PathBuf>,
    #[serde(default)]
    pub config_file: Option<PathBuf>,
}
