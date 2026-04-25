use serde::{Deserialize, Serialize};

/// The AI agent info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub name: String,
    pub description: String,
}
