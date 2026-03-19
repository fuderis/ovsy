use agent::macron::{Display, Error, From};

// The error
#[derive(Debug, Display, Error, From)]
pub enum Error {
    #[from]
    Io(std::io::Error),

    #[display = "Playlist '{0}' is not found."]
    PlaylistNotFound(String),

    #[display = "Pactl set-volume failed: {0}"]
    PactlError(std::io::Error),

    #[display = "Audio devices not found"]
    DevicesNotFound,
}
