use crate::prelude::*;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::AsyncWriteExt;

/// The user-query session logger
#[derive(Debug)]
pub struct SessionLogger {
    session_id: String,
    start_time: Option<Instant>,
    file: Option<File>,
}

impl SessionLogger {
    /// Create a new instance of session logger
    pub async fn new<S>(session_id: S, query: &str) -> Result<Self>
    where
        S: Into<String>,
    {
        let session_id = session_id.into();
        let session_dir = app_data().join("sessions").join(&session_id);
        let session_logs_dir = session_dir.join("logs");
        fs::create_dir_all(&session_logs_dir).await?;

        // create file:
        let timestamp = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
        let mut file_name = fmt!("{timestamp}.log");
        for (f, t) in [("T", "_"), ("Z", ""), (":", "-")] {
            file_name = file_name.replace(f, t);
        }
        let file_path = session_logs_dir.join(file_name);

        // open file:
        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&file_path)
            .await?;

        // write header:
        f.write_all(fmt!("Timestamp: {timestamp}").as_bytes())
            .await?;
        f.write_all(fmt!("\nQuery: {}", query.replace("\n", "\\n")).as_bytes())
            .await?;
        f.flush().await?;

        info!("Initialized session log on {file_path:?}");

        Ok(Self {
            session_id,
            start_time: Some(Instant::now()),
            file: Some(f),
        })
    }

    /// Writes 'tool calls' section
    pub async fn write_tool_calls(&mut self, tool_calls: &[ToolCall]) -> Result<()> {
        let f = self.file.as_mut().expect("File not initialized");

        f.write_all(
            fmt!(
                "\nTool calls: {}",
                json::to_string(&tool_calls).unwrap().replace("\n", "\\n")
            )
            .as_bytes(),
        )
        .await?;
        f.write_all(b"\n\n").await?;
        f.flush().await?;

        Ok(())
    }

    /// Add line to response output
    pub async fn write(&mut self, chunk: &str) -> Result<()> {
        let f = self.file.as_mut().expect("File not initialized");

        let chunk_bytes = chunk.as_bytes().to_vec();
        f.write_all(&chunk_bytes).await?;
        f.flush().await?;

        Ok(())
    }

    /// Finishes the query session & writes eof
    pub async fn finish(&mut self) -> Result<()> {
        let mut f = self.file.take().expect("File not initialized");
        let duration_ms = self.start_time.unwrap().elapsed().as_millis();

        f.write_all(fmt!("\n\nDuration: {duration_ms} ms").as_bytes())
            .await?;
        f.write_all("\n[EOF]".as_bytes()).await?;
        f.flush().await?;

        info!(
            "Session '{}' query finished by {duration_ms} ms",
            self.session_id
        );
        Ok(())
    }
}
