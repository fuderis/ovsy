use macron::{Display, Error, From};
use std::path::PathBuf;

/// The result alias
pub type Result<T> = macron::Result<T>;
pub type StdResult<T, E> = std::result::Result<T, E>;

// The error
#[derive(Debug, Display, Error, From)]
pub enum Error {
    #[from]
    Io(std::io::Error),

    #[display = "Database connection is not initialized"]
    DatabaseConnect,

    #[display = "Failed to generate query embeddings"]
    Embeddings,

    #[display = "Failed to read tools dir {0:?}: {1}"]
    ToolsDirRead(PathBuf, std::io::Error),
    #[display = "Failed to read manifest {0:?}: {1}"]
    ManifestRead(PathBuf, String),
    #[display = "Failed to parse manifest {0:?}: {1}"]
    ManifestParse(PathBuf, toml::de::Error),

    #[display = "Agent named as '{0}' is not found."]
    UnexpectedAgentName(String),
    #[display = "Failed to start tool server: {0}"]
    RunAgentServer(String, std::io::Error),

    #[display = "Tool handled with status '{0}': {1}"]
    AgentBadStatus(u16, String),
    #[display = "Tool exec '{0}' failed: {1}"]
    AgentExec(String, String),
    #[display = "Agent '{0}' failed health check"]
    AgentHealth(String),

    #[display = "Client disconnected, aborting tool chain"]
    ClientDisconnected,
    #[display = "Recursion limit, interrupting handling"]
    RecursionLimit,

    #[display = "Agent execution error: {0}"]
    ExecutionStop(Box<macron::DynError>),
}
