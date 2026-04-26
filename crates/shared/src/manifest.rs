use anylm::Schema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The AI agent options
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AgentOptions {
    pub name: String,
    pub description: String,
    pub exec_path: PathBuf,
}

/// The AI agent manifest
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub agent: AgentOptions,
    pub tools: Vec<Schema>,
}
