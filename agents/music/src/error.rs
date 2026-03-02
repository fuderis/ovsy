use macron::{Display, Error, From};

/// The result alias
pub type Result<T> = macron::Result<T>;
pub type StdResult<T, E> = std::result::Result<T, E>;

// The error
#[derive(Debug, Display, Error, From)]
pub enum Error {
    #[from]
    Io(std::io::Error),

    #[display = "Expected server '--port' argument."]
    ExpectedPortArg,

    #[display = "Playlist '{0}' is not found."]
    PlaylistNotFound(String),
}
