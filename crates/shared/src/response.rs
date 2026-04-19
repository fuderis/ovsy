use serde::{Deserialize, Serialize};

/// The /refresh response structure
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RefreshResponse {
    Error { error: String },
    Success { agents: Vec<(String, bool)> },
}
