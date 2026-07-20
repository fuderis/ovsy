use crate::prelude::*;

/// The session metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Metadata {
    pub session_id: SessionId,
    pub message_count: u64,
    pub compressed_until: usize,
}

impl Metadata {
    /// Creates a new session metadata by session id
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            message_count: 0,
            compressed_until: 0,
        }
    }
}
