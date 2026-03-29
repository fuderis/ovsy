use crate::prelude::*;

/// The cached user query data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedQuery {
    pub timestamp: DateTime<Utc>,
    pub query_len: usize,
    pub agent_name: String,
    #[serde(default, rename = "_distance")]
    pub distance: Option<f32>,
}

impl CachedQuery {
    /// Creates a cached data from query vector
    pub fn new(query_len: usize, agent_name: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            query_len,
            agent_name: agent_name.into(),
            distance: None,
        }
    }
}
