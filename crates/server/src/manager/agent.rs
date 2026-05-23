use super::Manager;
use crate::prelude::*;

use ovsy_shared::Manifest;
use tokio::process::{Child, Command};

/// The AI agent
#[derive(Default, Debug, Clone)]
pub struct Agent {
    pub dir: PathBuf,
    pub port: u16,
    pub manifest: Config<Manifest>,
    _child: Arc<Mutex<Option<Child>>>,
}

impl Agent {
    /// Runs the agent server
    pub async fn run(dir: impl Into<PathBuf>) -> Result<Option<Self>> {
        let dir = dir.into();

        // read manifest:
        let manif_path = dir.join("Ovsy.toml");
        let manifest = Config::<Manifest>::new(&manif_path).await.map_err(|e| {
            error!("Failed to parse manifest `{}`: {e}", manif_path.display());
            e
        })?;

        // check agent for already running:
        if Manager::contains(&arc!(manifest.agent.name.clone())).await {
            return Ok(None);
        }

        // run agent server:
        let exec_path = dir.join(&str!(
            "{}{}-agent",
            &manifest.agent.name,
            if cfg!(windows) { ".exe" } else { "" }
        ));
        let port = crate::free_port().await?;
        let child = Command::new(exec_path)
            .args(&["--port", &str!(port)])
            .args(&["--max-logs", &str!(Settings::get().server.max_logs)])
            .kill_on_drop(true)
            .spawn()?;

        let agent = Self {
            dir,
            port,
            manifest,
            _child: arc_mutex!(Some(child)),
        };

        Ok(Some(agent))
    }

    /// Returns true if agent is needs to be restarted
    pub async fn check(&self) -> Result<bool> {
        let manif_path = self.dir.join("Ovsy.toml");
        Ok(!manif_path.is_file() || self.manifest.check(0).await?)
    }
}
