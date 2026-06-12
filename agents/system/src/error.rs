use crate::{prelude::DynError, volume::VolumeMode};
use macron::{Display, Error, From};

// The error
#[derive(Debug, Display, Error, From)]
pub enum Error {
    #[display(fmt = "A non-existing tool was called: `{0}`")]
    UnknownTool(String),

    #[display(fmt = "Audio devices not found")]
    DevicesNotFound,

    #[from(skip)]
    #[display(fmt = "Set volume failed: {0}")]
    SetVolume(DynError),

    #[from(skip)]
    #[display(fmt = "Get volume failed: {0}")]
    GetVolume(DynError),

    #[from(skip)]
    #[display(fmt = "Get mute status failed: {0}")]
    GetMute(DynError),

    #[from(skip)]
    #[display(fmt = "Get mute volume failed: {0}")]
    SetMute(DynError),

    #[from(skip)]
    #[display(fmt = "Value is required for '{0}' mode")]
    ExpectedValue(VolumeMode),
}
