use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The user session info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Working directory where the client started
    pub current_path: Option<PathBuf>,

    /// Client timezone offset in minutes
    pub timezone: i16,
}
