use serde::{Deserialize, Serialize};

/// The agent skill info
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
}
