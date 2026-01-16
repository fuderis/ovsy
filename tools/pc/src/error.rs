use macron::{ Display, From, Error };

/// Std Result alias
pub type StdResult<T, E> = std::result::Result<T, E>;
/// Result alias
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

// The error
#[derive(Debug, Display, Error, From)]
pub enum Error {
    #[from]
    Io(std::io::Error),

    #[display = "Expected server '--port' argument."]
    ExpectedPortArg,
    
    #[display = "Playlist '{0}' is not found."]
    PlaylistNotFound(String),
    #[display = "Album '{0}' is not found in '{1}'."]
    AlbumNotFound(String, String),
}
