use anylm::Schema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The AI agent manifest
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub description: String,
    pub exec_path: PathBuf,
    pub tools: Vec<Schema>,
}
