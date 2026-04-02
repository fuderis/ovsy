use super::*;
use crate::prelude::*;
use tokio::fs;

const CACHE_TABLE: &'static str = "cache";

// Formatting:
const THINKING_COLOR: &'static str = "\x1b[36m"; // Cyan
const INFO_COLOR: &'static str = "\x1b[0m"; // White (default)
const ANSWER_COLOR: &'static str = "\x1b[92m"; // Green
const ERROR_COLOR: &'static str = "\x1b[31;1m"; // Red
const RESET: &'static str = "\x1b[0m"; // Default

/// The user session manager
pub struct Session {
    session_id: String,
    query: String,
    query_vector: Option<Vec<f32>>,
    context: Vec<String>,
    start_time: Instant,
    tx: StreamSender<Bytes>,
    is_thinking: bool,
    already_cached: bool,
}

impl Session {
    /// Creates a new session and generates the query embedding inside!
    pub async fn init(session_id: String, query: String, tx: StreamSender<Bytes>) -> Self {
        // converting query to embeddings:
        let query_vector = utils::to_embeddings(&query).await.ok().take().unwrap();

        Self {
            session_id,
            query,
            query_vector,
            context: Vec::new(),
            start_time: Instant::now(),
            tx,
            is_thinking: false,
            already_cached: false,
        }
    }

    /// Fast cache lookup. Generates embeddings and searches LanceDB.
    pub async fn load_cache(&mut self) -> Result<Option<String>> {
        if !Settings::get().cache.enable {
            return Ok(None);
        }

        if self.query_vector.is_none() {
            return Ok(None);
        }

        info!("[Cache] Checking cache for session: {}", self.session_id);
        let coef = Settings::get().cache.coefficient;

        // looking for up to 10 candidates:
        let cached_results: Option<Vec<Record<CachedQuery>>> = Database::read(
            CACHE_TABLE,
            self.query_vector.as_ref().unwrap().clone(),
            10,
            coef,
        )
        .await?;

        let Some(records) = cached_results else {
            info!("[Cache] No similar queries found in database.");
            return Ok(None);
        };

        info!("[Cache] Found {} potential cache candidates", records.len());

        for record in records {
            let agent_name = &record.data.agent_name;

            if Agents::get(agent_name).await.is_some() {
                info!(
                    "[Cache] Valid cache found! Agent '{}' is active.",
                    agent_name
                );
                self.already_cached = true;
                return Ok(Some(agent_name.clone()));
            } else {
                info!(
                    "[Cache] Agent '{}' no longer exists. Deleting record #{}",
                    agent_name, record.id
                );
                Database::delete(CACHE_TABLE, record.id).await.ok();
            }
        }

        info!("[Cache] All found candidates belonged to non-existent agents.");

        Ok(None)
    }

    /// The main method of processing a chunk
    pub async fn push(&mut self, chunk: SessionChunk) -> Result<()> {
        match chunk {
            SessionChunk::Thinking { thinking } => {
                if !self.is_thinking {
                    self.send_raw(&fmt!(
                        "{THINKING_COLOR}\x1b[1m[thinking]{RESET}{THINKING_COLOR}"
                    ));
                    self.is_thinking = true;
                }

                let formatted = thinking
                    .replace(
                        " Handling",
                        &fmt!("\x1b[1m Handling\x1b[0m{THINKING_COLOR}"),
                    )
                    .replace(
                        " Processing",
                        &fmt!("\x1b[1m Processing\x1b[0m{THINKING_COLOR}"),
                    );

                self.tx
                    .send(Bytes::from(fmt!("\n{THINKING_COLOR}{formatted}{RESET}")))
                    .ok();
                self.context.push(thinking);
            }

            SessionChunk::Answer { answer } => {
                self.close_thinking();
                self.tx
                    .send(Bytes::from(fmt!("\n{ANSWER_COLOR}{answer}{RESET}")))
                    .ok();
            }

            SessionChunk::Error { error, message } => {
                self.close_thinking();
                self.send_raw(&fmt!("\n{ERROR_COLOR}❌ Error: {message}: {error}{RESET}"));
                self.save_error(&error).await.ok();
                return Err(Error::ExecutionStop(Box::new(error.into())).into());
            }

            SessionChunk::Info { info } => {
                self.close_thinking();
                self.tx
                    .send(Bytes::from(fmt!("\n{INFO_COLOR}{info}{RESET}")))
                    .ok();
            }
        }

        Ok(())
    }

    /// Closes the thinking block
    fn close_thinking(&mut self) {
        if self.is_thinking {
            self.send_raw(&fmt!("\n{THINKING_COLOR}\x1b[1m[/thinking]{RESET}\n"));
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

    /// Finalizes the success and saves data without extra neural network calls!
    pub async fn finalize_success(&mut self, agent_name: Option<String>) -> Result<()> {
        if self.is_thinking {
            self.close_thinking();
        }

        // caching results:
        if !self.already_cached && Settings::get().cache.enable {
            if let Some(name) = agent_name
                && let Some(vector) = self.query_vector.take()
            {
                info!("[Cache] Saving new successful query to cache...");

                let cached_query = CachedQuery::new(self.query.len(), name);

                Database::write(CACHE_TABLE, vector, cached_query).await?;
                info!("[Cache] Cached data successfully saved to LanceDB!");
            }
        }

        Ok(())
    }

    /// Saving the error to a file
    async fn save_error(&self, error_detail: &str) -> Result<()> {
        // generating file name:
        let timestamp = Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        let file_name = fmt!("{}__{}.json", timestamp, self.session_id);

        let path = app_data().join("errors").join(file_name);

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
