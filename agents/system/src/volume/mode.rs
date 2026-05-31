use crate::prelude::*;

/// The power mode
#[derive(Debug, Clone, Copy, Display, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VolumeMode {
    #[display = "get"]
    Get,

    #[display = "set"]
    Set,

    #[display = "add"]
    Add,

    #[display = "mute"]
    Mute,

    #[display = "unmute"]
    Unmute,
}
