use super::Manager;
use crate::prelude::*;

use ovsy_share::AgentInfo;
use reqwest::Client;
use std::time::SystemTime;
use tokio::process::{Child, Command};

/// The AI agent
#[derive(Default, Debug, Clone)]
pub struct Agent {
    pub dir: PathBuf,
    pub exec_path: PathBuf,
    pub port: u16,
    pub info: AgentInfo,
    _started: Option<SystemTime>,
    _child: Arc<Mutex<Option<Child>>>,
}

impl Agent {
    /// Runs the agent server
    pub async fn run(dir: impl Into<PathBuf>) -> Result<Option<Self>> {
        let dir = dir.into();
        let name = dir
            .file_name()
            .ok_or(Error::FailedGetAgentName)?
            .to_string_lossy()
            .to_string();

        // check agent for already running:
        if Manager::contains(&arc!(name.clone())).await {
            return Ok(None);
        }

        // run agent server:
        let exec_path = dir.join(&str!(
            "{name}-agent{exe}",
            exe = if cfg!(windows) { ".exe" } else { "" }
        ));
        let port = crate::free_port().await?;
        let child = Command::new(&exec_path)
            .args(&["--port", &str!(port)])
            .args(&["--max-logs", &str!(Settings::get().server.max_logs)])
            .kill_on_drop(true)
            .spawn()?;

        // get agent info:
        let response = Client::new()
            .post(str!("http://127.0.0.1:{port}/info"))
            .send()
            .await?;
        let info = response.json::<AgentInfo>().await?;

        let agent = Self {
            dir,
            exec_path,
            port,
            info,
            _started: Some(SystemTime::now()),
            _child: arc_mutex!(Some(child)),
        };

        Ok(Some(agent))
    }

    /// Returns true if needs to be updated
    pub async fn check(&self) -> Result<bool> {
        let metadata = tokio::fs::metadata(&self.exec_path).await?;

        if let Ok(modified_at) = metadata.modified()
            && let Some(started_at) = self._started
        {
            Ok(modified_at > started_at)
        } else {
            Ok(false)
        }
    }
}
