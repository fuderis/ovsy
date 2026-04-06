use crate::prelude::*;

/// The session response chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SessionChunk {
    Thinking { thinking: String },
    Answer { answer: String },
    Error { error: String, message: String },
    Info { info: String },
}

impl SessionChunk {
    /// Creates a `thinking` chunk
    pub fn think(s: impl Into<String>) -> Self {
        Self::Thinking { thinking: s.into() }
    }

    /// Creates an `answer` chunk
    pub fn answer(s: impl Into<String>) -> Self {
        Self::Answer { answer: s.into() }
    }

    /// Creates an `error` chunk
    pub fn error(e: impl Into<String>, s: impl Into<String>) -> Self {
        Self::Error {
            error: e.into(),
            message: s.into(),
        }
    }

    /// Creates an `info` chunk
    pub fn info(s: impl Into<String>) -> Self {
        Self::Info { info: s.into() }
    }

    /// Converts the chunk to string
    pub fn to_string(&self) -> String {
        // SAFETY: It's will never panic
        json::to_string(&self).unwrap()
    }
}
