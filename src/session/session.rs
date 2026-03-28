use super::SessionChunk;
use crate::prelude::*;
use tokio::fs;

/// The user session manager
pub struct Session {
    session_id: String,
    query: String,
    context: Vec<String>,
    start_time: Instant,
    tx: StreamSender<Bytes>,
    is_thinking: bool,
}

impl Session {
    /// Creates a new session by user id
    pub fn new(session_id: String, query: String, tx: StreamSender<Bytes>) -> Self {
        Self {
            session_id,
            query,
            context: Vec::new(),
            start_time: Instant::now(),
            tx,
            is_thinking: false,
        }
    }

    /// The main method of processing a chunk
    pub async fn push(&mut self, chunk: SessionChunk) -> Result<()> {
        match chunk {
            SessionChunk::Thinking { thinking } => {
                if !self.is_thinking {
                    self.send_raw("\x1b[34;1m[thinking]\x1b[0m\x1b[34m\n");
                    self.is_thinking = true;
                }

                let formatted = thinking
                    .replace(" Handling", "\x1b[1m Handling\x1b[0;34m")
                    .replace(" Processing", "\x1b[1m Processing\x1b[0;34m");

                self.tx
                    .send(Bytes::from(fmt!("\x1b[34m{formatted}\x1b[0m")))
                    .ok();
                self.context.push(thinking);
            }

            SessionChunk::Answer { answer } => {
                self.close_thinking();
                self.tx
                    .send(Bytes::from(fmt!("\x1b[97m{answer}\x1b[0m")))
                    .ok();
            }

            SessionChunk::Error { error, message } => {
                self.close_thinking();
                self.send_raw(&fmt!("\n\x1b[31;1m❌ Error: {message}\x1b[0m\n"));
                self.save_error(&error).await.ok();
                return Err(Error::ExecutionStop(Box::new(error.into())).into());
            }

            SessionChunk::Info { info } => {
                self.close_thinking();
                self.tx
                    .send(Bytes::from(fmt!("\x1b[92m{info}\x1b[0m")))
                    .ok();
            }
        }

        Ok(())
    }

    /// Closes the thinking block
    fn close_thinking(&mut self) {
        if self.is_thinking {
            self.send_raw("\n\x1b[34;1m[/thinking]\x1b[0m\x1b[34m\n\n");
            self.is_thinking = false;
        }
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

    /// Send raw str to Sender
    pub fn send_raw(&self, data: &str) {
        let _ = self.tx.send(Bytes::copy_from_slice(data.as_bytes()));
    }

    /// Saving results to RAG DB
    pub async fn finalize_success(&mut self) -> Result<()> {
        if self.is_thinking {
            self.close_thinking();
        }
        info!("Saving session {} to RAG DB...", self.session_id);

        // TODO: Logic for RAG DB

        Ok(())
    }

    /// Saving the error to a file
    async fn save_error(&self, error_detail: &str) -> Result<()> {
        let path = app_data()
            .join("errors")
            .join(fmt!("{}.json", self.session_id));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let report = json!({
            "session_id": self.session_id,
            "query": self.query,
            "error": error_detail,
            "context": self.context,
            "timestamp": Utc::now()
        });

        fs::write(path, json::to_string_pretty(&report)?).await?;

        Ok(())
    }

    /// Returns true if connection with client is closed
    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }

    /// Calculates the execution time
    pub fn exec_time(&self) -> u128 {
        self.start_time.elapsed().as_millis()
    }

    /// Returns the current results
    pub fn results(&self) -> &Vec<String> {
        &self.context
    }
}
