use crate::prelude::*;

/// The power mode
#[derive(Debug, Clone, Copy, Display, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VolumeMode {
    #[display(fmt = "get")]
    Get,

    #[display(fmt = "set")]
    Set,

    #[display(fmt = "add")]
    Add,

    #[display(fmt = "mute")]
    Mute,

    #[display(fmt = "unmute")]
    Unmute,
}
