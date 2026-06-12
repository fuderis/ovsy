use crate::SessionID;
use anylm::Message;
use serde::{Deserialize, Serialize};

/// The user query data (/handle action)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleQuery {
    pub session_id: SessionID,
    pub message: Message,
}

impl HandleQuery {
    /// Creates a new user query
    pub fn new(session_id: SessionID, message: Message) -> Self {
        Self {
            session_id,
            message,
        }
    }
}
