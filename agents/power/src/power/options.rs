use crate::{PowerMode, prelude::*};

/// The system power options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerOptions {
    #[serde(rename = "power_mode")]
    pub mode: PowerMode,
    #[serde(default, rename = "timeout_seconds")]
    pub timeout: u64,
}
