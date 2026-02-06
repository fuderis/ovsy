use macron::{Display, Error, From};
use std::path::PathBuf;

/// Std Result alias
pub type StdResult<T, E> = std::result::Result<T, E>;
/// Result alias
pub type Result<T> = std::result::Result<T, DynError>;
/// Error alias
pub type DynError = Box<dyn std::error::Error + Send + Sync + 'static>;

// The error
#[derive(Debug, Display, Error, From)]
pub enum Error {
    #[from]
    Io(std::io::Error),

    #[display = "Failed read tools dir {0:?}: {1}"]
    ToolsDirRead(PathBuf, std::io::Error),
    #[display = "Failed read manifest {0:?}: {1}"]
    ManifestRead(PathBuf, String),
    #[display = "Failed parse manifest {0:?}: {1}"]
    ManifestParse(PathBuf, toml::de::Error),

    #[display = "Invalid tool call name '{0}', expected format: '{{name}}/{{action}}'."]
    InvalidToolNameFormat(String),
    #[display = "Tool named as '{0}' is not found."]
    UnexpectToolName(String),
    #[display = "Failed to start tool server: {0}"]
    FailedRunTool(String, std::io::Error),

    #[display = "Tool handled with status '{0}': {1}"]
    ToolBadStatus(u16, String),
    #[display = "Tool exec '{0}' failed: {1}"]
    ToolExecFailed(String, String),

    #[display = "Client disconnected, aborting tool chain"]
    ClientDisconnected,
}
