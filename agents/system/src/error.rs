use crate::{prelude::DynError, volume::VolumeMode};
use macron::{Display, Error, From};

// The error
#[derive(Debug, Display, Error, From)]
pub enum Error {
    #[display = "A non-existing tool was called: `{0}`"]
    UnknownTool(String),

    #[display = "Audio devices not found"]
    DevicesNotFound,

    #[display = "Set volume failed: {0}"]
    SetVolume(DynError),

    #[display = "Get volume failed: {0}"]
    GetVolume(DynError),

    #[display = "Get mute status failed: {0}"]
    GetMute(DynError),

    #[display = "Get mute volume failed: {0}"]
    SetMute(DynError),

    #[display = "Value is required for '{0}' mode"]
    ExpectedValue(VolumeMode),
}
