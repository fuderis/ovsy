use crate::SessionID;
use serde::{Deserialize, Serialize};

/// The user query data (/compact action)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactQuery {
    pub session_id: SessionID,
    pub preserve: usize,
}

impl CompactQuery {
    /// Creates a new user query
    pub fn new(session_id: SessionID, preserve: usize) -> Self {
        Self {
            session_id,
            preserve,
        }
    }
}
