use crate::SessionID;
use serde::{Deserialize, Serialize};

/// The user query data (/clear action)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearQuery {
    pub session_id: SessionID,
}

impl ClearQuery {
    /// Creates a new user query
    pub fn new(session_id: SessionID) -> Self {
        Self { session_id }
    }
}
