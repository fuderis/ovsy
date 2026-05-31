use anylm::Tool;
use serde::{Deserialize, Serialize};

/// The AI agent info
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub name: String,
    pub description: String,
    pub prompt: String,
    pub tools: Vec<Tool>,
}
