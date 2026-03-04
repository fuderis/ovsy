use macron::{Display, Error, From};

/// The result alias
pub type Result<T> = macron::Result<T>;
pub type StdResult<T, E> = std::result::Result<T, E>;

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
