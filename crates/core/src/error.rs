use crate::prelude::DynError;
use macron::{Display, Error, From};

// The error
#[derive(Debug, Display, Error, From)]
pub enum Error {
    #[display(fmt = "Failed to get agent name (incorrect dir path)")]
    FailedGetAgentName,

    #[display(fmt = "Agent `{name}` failed to start on port {port} after 10 attempts.")]
    AgentStartFailed { name: String, port: u16 },

    #[display(fmt = "Failed to parse AgentInfo response payload: {0}")]
    AgentInfoParsingFailed(DynError),
}
