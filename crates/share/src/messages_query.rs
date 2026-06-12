use crate::SessionID;
use serde::{Deserialize, Serialize};

/// The user query data (/messages action)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagesQuery {
    pub session_id: SessionID,
}

impl MessagesQuery {
    /// Creates a new user query
    pub fn new(session_id: SessionID) -> Self {
        Self { session_id }
    }
}
