use crate::prelude::DynError;
use macron::{Display, Error, From};

// The error
#[derive(Debug, Display, Error, From)]
pub enum Error {
    #[display(fmt = "Failed to get agent name (incorrect dir path)")]
    FailedGetAgentName,

    #[display(fmt = "Agent `{name}` failed to start on sock {sock_path} after 10 attempts.")]
    AgentStartFailed { name: String, sock_path: String },

    #[display(fmt = "Failed to parse AgentInfo response payload: {0}")]
    AgentInfoParsingFailed(#[source] DynError),

    #[display(fmt = "The TypeScript runtime is not initialized, check logs")]
    RuntimeNotInitialized,
}
