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

    #[display(fmt = "Unsupported operating system")]
    UnsupportedOS,

    #[cfg(target_os = "linux")]
    #[display(fmt = "Failed to execute gsettings: {0}")]
    GsettingsExecute(std::io::Error),

    #[cfg(target_os = "linux")]
    #[display(fmt = "gsettings exited with non-zero status")]
    GsettingsExitStatus,

    #[cfg(target_os = "macos")]
    #[display(fmt = "Failed to execute osascript: {0}")]
    OsascriptExecute(std::io::Error),

    #[cfg(target_os = "macos")]
    #[display(fmt = "osascript exited with non-zero status")]
    OsascriptExitStatus,

    #[cfg(target_os = "windows")]
    #[from(skip)]
    #[display(fmt = "Task join error: {0}")]
    TaskJoin(DynError),

    #[cfg(target_os = "windows")]
    #[display(fmt = "Failed to write Windows registry: {0}")]
    Registry(std::io::Error),
}
