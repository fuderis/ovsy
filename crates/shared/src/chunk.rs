use crate::AgentTask;
use anylm::{Bytes, ToolCall};
use serde::{Deserialize, Serialize};

/// The response chunk data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkData {
    Tools(Vec<ToolCall>),
    Thinking(String),
    Answer(String),
    Error(String),
    Finish,
}

/// The response chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub agent: Option<AgentTask>,
    pub data: ChunkData,
}

impl Chunk {
    /// Creates a new chunk
    pub fn new(data: ChunkData) -> Self {
        Self { agent: None, data }
    }

    /// Sets the full agent info
    pub fn task_info(mut self, info: AgentTask) -> Self {
        self.agent.replace(info);
        self
    }

    /// Creates a tools chunk
    pub fn tools(tool_calls: Vec<ToolCall>) -> Self {
        Self::new(ChunkData::Tools(tool_calls))
    }

    /// Creates a thinking chunk
    pub fn think(msg: impl Into<String>) -> Self {
        Self::new(ChunkData::Thinking(msg.into()))
    }

    /// Creates an answer chunk
    pub fn answer(msg: impl Into<String>) -> Self {
        Self::new(ChunkData::Answer(msg.into()))
    }

    /// Creates an error chunk
    pub fn error(msg: impl Into<String>) -> Self {
        Self::new(ChunkData::Error(msg.into()))
    }

    /// Creates a final agent chunk
    pub fn finish() -> Self {
        Self::new(ChunkData::Finish)
    }

    /// Converts the chunk to string
    pub fn to_string(&self) -> String {
        // SAFETY: will be never panic
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
