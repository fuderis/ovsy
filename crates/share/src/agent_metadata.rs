use crate::Skill;
use serde::{Deserialize, Serialize};

/// The agent metadata
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetadata {
    pub name: String,
    pub description: String,
    pub version: String,
    pub prompt: String,
    pub skills: Vec<Skill>,
}
