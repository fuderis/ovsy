pub mod chunk;
pub use chunk::SessionChunk;
pub mod cached;
pub use cached::CachedQuery;

use crate::prelude::*;
use tokio::fs;

const CACHE_TABLE: &'static str = "cache";

// Formatting:
/* ========= TERMINAL FORMATTING QUICK REFERENCE ============
Using: "\x1b[<color>m" or "\x1b[<style>;<color>m"
Styles: 0 = Normal, 1 = Bold, 2 = Dim, 4 = Underline

Code Color Hex        | Code Color Hex
-------------------------------------------------------------
30 Black   #000000    | 90 Dark Gray #666666
31 Red     #cd3131    | 91 Light Red #f14c4c
32 Green   #0dbc79    | 92 Light Green #23d18b
33 Yellow  #e5e510    | 93 Light Yellow #f5f543
34 Blue    #2472c8    | 94 Light Blue #3b8eea
35 Magenta #bc3fbc    | 95 Light Magenta #d670d6
36 Cyan    #11a8cd    | 96 Light Cyan #29b8db
37 White   #e5e5e5    | 97 Pure White #ffffff
============================================================= */
const THINKING_FORMATTING: &'static str = "\x1b[36m"; // Cyan #11a8cd
const INFO_FORMATTING: &'static str = "\x1b[0m"; // Default #ffffff
const ANSWER_FORMATTING: &'static str = "\x1b[92m"; // Green #23d18b
const ERROR_FORMATTING: &'static str = "\x1b[31;1m"; // Red #cd3131
const RESET_FORMATTING: &'static str = "\x1b[0m"; // Default #ffffff

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

        info!("Checking cache for session: {}", self.session_id);
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
            info!("No similar queries found in database.");
            return Ok(None);
        };

        info!("Found {} potential cache candidates", records.len());

        for record in records {
            let agent_name = &record.data.agent_name;

            if Agents::get(agent_name).await.is_some() {
                info!("Valid cache found! Agent '{}' is active.", agent_name);
                self.already_cached = true;
                return Ok(Some(agent_name.clone()));
            } else {
                info!(
                    "Agent '{}' no longer exists. Deleting record #{}",
                    agent_name, record.id
                );
                Database::delete(CACHE_TABLE, record.id).await.ok();
            }
        }

        info!("All found candidates belonged to non-existent agents.");

        Ok(None)
    }

    /// The main method of processing a chunk
    pub async fn push(&mut self, chunk: SessionChunk) -> Result<()> {
        match chunk {
            SessionChunk::Thinking { thinking } => {
                if !self.is_thinking {
                    self.send_raw(&str!(
                        "{THINKING_FORMATTING}\x1b[1m[thinking]{RESET_FORMATTING}{THINKING_FORMATTING}"
                    ));
                    self.is_thinking = true;
                }

                let formatted = thinking
                    .replace(
                        " Handling",
                        &str!("\x1b[1m Handling\x1b[0m{THINKING_FORMATTING}"),
                    )
                    .replace(
                        " Processing",
                        &str!("\x1b[1m Processing\x1b[0m{THINKING_FORMATTING}"),
                    );

                self.tx
                    .send(Bytes::from(str!(
                        "\n{THINKING_FORMATTING}{formatted}{RESET_FORMATTING}"
                    )))
                    .ok();
                self.context.push(thinking);
            }

            SessionChunk::Answer { answer } => {
                self.close_thinking();
                self.tx
                    .send(Bytes::from(str!(
                        "\n{ANSWER_FORMATTING}{answer}{RESET_FORMATTING}"
                    )))
                    .ok();
            }

            SessionChunk::Error { error, message } => {
                self.close_thinking();
                self.send_raw(&str!(
                    "\n{ERROR_FORMATTING}❌ Error: {message}: {error}{RESET_FORMATTING}"
                ));
                self.save_error(&error).await.ok();
                return Err(Error::ExecutionStop(Box::new(error.into())).into());
            }

            SessionChunk::Info { info } => {
                self.close_thinking();
                self.tx
                    .send(Bytes::from(str!(
                        "\n{INFO_FORMATTING}{info}{RESET_FORMATTING}"
                    )))
                    .ok();
            }
        }

        Ok(())
    }

    /// Closes the thinking block
    fn close_thinking(&mut self) {
        if self.is_thinking {
            self.send_raw(&str!(
                "\n{THINKING_FORMATTING}\x1b[1m[/thinking]{RESET_FORMATTING}\n"
            ));
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
                info!("Saving a new successful query to cache...");

                let cached_query = CachedQuery::new(self.query.len(), name);

                Database::write(CACHE_TABLE, vector, cached_query).await?;
                info!("Cached data successfully saved!");
            }
        }

        Ok(())
    }

    /// Saving the error to a file
    async fn save_error(&self, error_detail: &str) -> Result<()> {
        // generating file name:
        let timestamp = Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        let file_name = str!("{}__{}.json", timestamp, self.session_id);

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
