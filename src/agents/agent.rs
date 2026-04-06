use super::Agents;
use crate::{Manifest, prelude::*};
use anylm::{Schema, Tool};
use reqwest::Client;
use std::{fs as stdfs, process::Stdio, time::SystemTime};
use tokio::process::{Child, Command};

/// Count of tries to catch server log file (1 failed try = 100ms wait)
const TRACE_LOG_FILE_TRIES: usize = 100;

/// The agent server /health response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub log_file: PathBuf,
}

/// The agent structure
#[derive(Default, Clone, Debug)]
pub struct Agent {
    pub dir: PathBuf,
    pub port: Option<u16>,
    pub manifest: Config<Manifest>,
    pub examples: Vec<String>,
    pub tools: Vec<Tool>,
    pub last_update: Option<SystemTime>,
    pub trace: Option<Trace>,
    child: Arc<Mutex<Option<Child>>>,
}

impl Agent {
    /// Reads an agent server & runs it
    pub(super) async fn run<P>(dir: P) -> Result<Option<()>>
    where
        P: AsRef<Path>,
    {
        let agent_dir = dir.as_ref();

        // read manifest:
        let manifest_path = agent_dir.join("Ovsy.toml");
        let manifest = match Config::<Manifest>::new(&manifest_path) {
            Ok(r) => r,
            Err(e) => {
                warn!("Read manifest '{}' error: {e}", manifest_path.display());
                return Ok(None);
            }
        };
        let agent_name = &manifest.agent.name;

        // check if agent already is running:
        if Agents::has(agent_name).await {
            trace!("Agent '{agent_name}' already running, skipping..");
            return Ok(None);
        }

        // remove agent if disabled:
        if !manifest.agent.enable {
            Agents::stop(agent_name).await?;
            return Ok(None);
        }

        // get agent exec file path:
        let exec_path = agent_dir.join({
            #[cfg(debug_assertions)]
            {
                &manifest.agent.debug_exec
            }
            #[cfg(not(debug_assertions))]
            {
                &manifest.agent.exec
            }
        });

        // read manifest mimetype:
        let mut last_update: Option<SystemTime> = if manifest_path.is_file() {
            Some(stdfs::metadata(&manifest_path)?.modified()?)
        } else {
            None
        };
        if exec_path.is_file() {
            let exec_mtime = stdfs::metadata(&exec_path)?.modified()?;
            match last_update {
                Some(current) => last_update = Some(current.max(exec_mtime)),
                None => last_update = Some(exec_mtime),
            }
        }

        // collect examples & tools:
        let mut examples = vec![];
        let mut tools = vec![];

        for (name, tool) in manifest.tools.iter() {
            // gen examples:
            for exmpl in tool.examples.iter() {
                examples.push(str!(
                    r#"* query: "{query}", result: {name} {data}"#,
                    query = exmpl.query,
                    data = json::to_string(&exmpl.data).unwrap(),
                ))
            }

            // create tool & push:
            let mut schema = Schema::object("");
            for (name, arg) in tool.arguments.iter() {
                let value = json::to_value(arg).unwrap();
                schema.set_property(
                    name.clone(),
                    json::from_value(value).unwrap(),
                    !arg.optional,
                );
            }
            tools.push(Tool::new(name.clone(), tool.description.clone(), schema));
        }

        // run tool server (if it's a server):
        let (port, child) = if manifest.agent.is_server {
            // bind free port:
            let port = utils::get_free_port().await?;

            // create command with --port argument:
            let mut cmd = Command::new(&exec_path);
            cmd.arg("--port").arg(port.to_string());
            cmd.stdout(Stdio::null());
            cmd.stderr(Stdio::null());
            cmd.kill_on_drop(true);

            // spawning the agent server process:
            let child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed start '{}' agent server: {e}", manifest.agent.name);
                    return Err(Error::RunAgentServer(manifest.agent.name.clone(), e).into());
                }
            };

            // waiting for running the server:
            sleep(Duration::from_millis(500)).await;

            (Some(port), Some(child))
        } else {
            // NOTE: So, this is a binary agent, no startup required..
            (None, None)
        };

        // register tool instance:
        let mut agent = Agent {
            dir: agent_dir.to_path_buf(),
            manifest,
            port,
            examples,
            tools,
            last_update,
            trace: None,
            child: Arc::new(Mutex::new(child)),
        };

        // capturing the server log file:
        agent.trace().await?;

        // pushing to the agents manager:
        Agents::add(agent).await;

        Ok(Some(()))
    }

    /// Checks agent server for alive & returns his answer
    pub async fn health(&self) -> Result<HealthResponse> {
        let client = Client::new();
        let agent_name = &self.manifest.agent.name;
        let health_url = str!("http://127.0.0.1:{}/health", self.port.as_ref().unwrap());

        if let Ok(resp) = client.post(&health_url).send().await {
            if resp.status().is_success() {
                // agent responded - parsing answer:
                let data = resp.json::<HealthResponse>().await?;
                return Ok(data);
            }
        }

        warn!("Agent '{agent_name}' failed health check");
        Err(Error::AgentHealth(agent_name.clone()).into())
    }

    /// Trace the agent server log file
    pub(super) async fn trace(&mut self) -> Result<()> {
        let agent_name = &self.manifest.agent.name;

        for n in 1..=TRACE_LOG_FILE_TRIES {
            info!("Trying to catch agent '{agent_name}' log file (attempt {n})..");

            if let Ok(data) = self.health().await {
                let path = data.log_file;
                self.trace
                    .replace(Trace::open(path, Duration::from_millis(500), false).await?);
                return Ok(());
            }

            sleep(Duration::from_millis(100)).await;
        }

        error!("Agent '{agent_name}' did not provide log_file path via /health");
        Err(Error::AgentHealth(agent_name.clone()).into())
    }

    /// Checks the status of the agent's server and restarts it if necessary
    pub(super) async fn check(&self) -> Result<()> {
        let agent_dir = &self.dir;
        let manifest_path = agent_dir.join("Ovsy.toml");
        let name = &self.manifest.agent.name;

        // ping server health:
        if self.health().await.is_err() {
            warn!("Connection to the '{name}' agent server is lost - restarting..");

            // restart server:
            self.restart().await?;
        }

        // check if manifest exists:
        if !manifest_path.exists() {
            warn!(
                "Manifest '{}' not found, stopping '{name}' agent server..",
                manifest_path.display()
            );
            Agents::stop(&self.manifest.agent.name).await?;
            return Ok(());
        }

        // check if still enabled:
        match Config::<Manifest>::new(&manifest_path) {
            Ok(new_manifest) => {
                if !new_manifest.agent.enable {
                    warn!("Agent '{name}' disabled in manifest, finishing work..");
                    Agents::stop(name).await?;
                    return Ok(());
                }
            }
            Err(e) => {
                warn!(
                    "Failed to read manifest '{}': {e}..",
                    manifest_path.display()
                );
                Agents::stop(name).await?;
                return Ok(());
            }
        }

        // get the manifest modify time:
        let exec_path = agent_dir.join(&self.manifest.agent.exec);
        let mut new_update: Option<SystemTime> = if manifest_path.exists() {
            Some(stdfs::metadata(&manifest_path)?.modified()?)
        } else {
            None
        };
        if exec_path.exists() {
            let exec_mtime = stdfs::metadata(&exec_path)?.modified()?;
            match new_update {
                Some(current) => new_update = Some(current.max(exec_mtime)),
                None => new_update = Some(exec_mtime),
            }
        }

        // rerun tool server (if outdated):
        match (self.last_update, new_update) {
            (Some(old), Some(new)) if new > old => {
                info!("Agent '{name}' outdated, restarting..");

                // restart server:
                self.restart().await?;
            }
            _ => {
                // up to date
                trace!("Agent '{name}' is up to date!");
            }
        }

        Ok(())
    }

    /// Restarts the agent server
    pub(super) async fn restart(&self) -> Result<()> {
        let agent_dir = &self.dir;
        let name = &self.manifest.agent.name;

        // stop server:
        Agents::stop(name).await?;

        // restart server:
        if let Some(()) = Self::run(agent_dir).await? {
            info!("Agent '{name}' successfully restarted");
        }

        Ok(())
    }

    /// Kills the agent server process
    pub(super) async fn stop(&self) -> Result<()> {
        if !self.manifest.agent.is_server {
            return Ok(());
        }

        let mut lock = self.child.lock().await;
        if let Some(mut child) = lock.take() {
            info!("Killing agent process '{}'...", self.manifest.agent.name);
            child.kill().await?;
            // waiting for kill process:
            let _ = child.wait().await;
        }

        Ok(())
    }
}
