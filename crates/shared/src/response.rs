use crate::AgentInfo;
use serde::{Deserialize, Serialize};

/// The /status response structure
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StatusResponse {
    Error { error: String },
    Success { agents: Vec<AgentInfo> },
}
