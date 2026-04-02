use crate::prelude::*;

/// The user session manager (JSON Streamer)
pub struct Session {
    tx: StreamSender<Bytes>,
}

impl Session {
    /// Creates a new session bound to a stream sender
    pub fn new(tx: StreamSender<Bytes>) -> Self {
        Self { tx }
    }

    /// The main method of processing and sending a chunk as JSON with a newline separator
    pub async fn push(&mut self, chunk: SessionChunk) -> Result<()> {
        let mut json_string = chunk.to_string();

        // adding a line break as a packet separator for the main server:
        json_string.push('\n');

        // sending a JSON string to the stream:
        self.tx.send(Bytes::from(json_string)).ok();

        // if a chunk arrives with an error - interrupt the agent execution:
        if let SessionChunk::Error { error, .. } = chunk {
            return Err(Error::ExecutionStop(Box::new(error.into())).into());
        }

        Ok(())
    }

    /// Push the `thinking` chunk
    pub async fn think(&mut self, s: impl Into<String>) -> Result<()> {
        self.push(SessionChunk::think(s)).await
    }

    /// Push the `answer` chunk
    pub async fn answer(&mut self, s: impl Into<String>) -> Result<()> {
        self.push(SessionChunk::answer(s)).await
    }

    /// Push the `error` chunk
    pub async fn error(&mut self, e: impl Into<String>, s: impl Into<String>) -> Result<()> {
        self.push(SessionChunk::error(e, s)).await
    }

    /// Push the `info` chunk
    pub async fn info(&mut self, s: impl Into<String>) -> Result<()> {
        self.push(SessionChunk::info(s)).await
    }

    /// Returns true if connection with the main server/client is closed
    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }
}
