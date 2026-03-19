use agent::macron::{Display, Error, From};

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
