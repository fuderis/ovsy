use anylm::Bytes;
use serde::{Deserialize, Serialize};

/// The response chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Chunk {
    Think { think: String },
    Answer { answer: String },
    Error { error: String },
}

impl Chunk {
    /// Creates a thinking chunk
    pub fn think(msg: impl Into<String>) -> Bytes {
        Self::Think { think: msg.into() }.to_bytes()
    }

    /// Creates an answer chunk
    pub fn answer(msg: impl Into<String>) -> Bytes {
        Self::Answer { answer: msg.into() }.to_bytes()
    }

    /// Creates an error chunk
    pub fn error(msg: impl Into<String>) -> Bytes {
        Self::Error { error: msg.into() }.to_bytes()
    }

    /// Converts the chunk to string
    pub fn to_string(&self) -> String {
        // SAFETY: will never panic
        serde_json::to_string(&self).unwrap()
    }

    /// COnverts the chunk to bytes
    pub fn to_bytes(&self) -> Bytes {
        self.to_string().into()
    }
}

impl Into<String> for Chunk {
    fn into(self) -> String {
        self.to_string()
    }
}

impl Into<Bytes> for Chunk {
    fn into(self) -> Bytes {
        self.to_bytes()
    }
}
