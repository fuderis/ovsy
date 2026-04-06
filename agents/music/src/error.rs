use agent::macron::{Display, Error, From, prelude::DynError};

// The error
#[derive(Debug, Display, Error, From)]
pub enum Error {
    #[from]
    Io(std::io::Error),

    #[display = "Playlist '{0}' is not found."]
    PlaylistNotFound(String),

    #[display = "Set volume failed: {0}"]
    SetVolume(Box<DynError>),

    #[display = "Get volume failed: {0}"]
    GetVolume(Box<DynError>),

    #[display = "Audio devices not found"]
    DevicesNotFound,
}
