pub mod agent;
pub use agent::Agent;

use crate::prelude::*;
use ovsy_shared::AgentInfo;
use tokio::task::JoinSet;

/// The agents manager state
pub static MANAGER: State<Manager> = State::new();

/// The agents manager
#[derive(Default, Debug, Clone)]
pub struct Manager {
    pub agents: HashMap<Arc<String>, Arc<Agent>>,
}

impl Manager {
    /// Runs the agents management
    pub async fn init() -> Result<()> {
        let scan_dir = app_data().join("agents");
        let mut set = JoinSet::new();

        // read agent dirs:
        let mut reader = Dir::read(scan_dir).await?;
        while let Some(entry) = reader.next_entry().await? {
            let path = entry.path();

            // skip non-agent dirs:
            if !path.is_dir() || !path.join("Ovsy.toml").is_file() {
                continue;
            }

            // run agent server:
            set.spawn(async move { Agent::run(path).await });
        }

        // check results:
        while let Some(task_res) = set.join_next().await {
            if let Err(e) = task_res {
                error!("Agent spawn task panicked: {e}");
            }
        }

        Ok(())
    }

    /// Returns true if agent with this name is already on running
    pub async fn contains(name: Arc<String>) -> bool {
        MANAGER.get().await.agents.contains_key(&name)
    }

    /// Returns the all agents list
    pub async fn agents_list() -> Vec<AgentInfo> {
        MANAGER
            .get()
            .await
            .agents
            .iter()
            .map(|(_, agent)| AgentInfo {
                name: agent.manifest.agent.name.clone(),
                description: agent.manifest.agent.description.clone(),
            })
            .collect()
    }

    /// Runs the agent server
    pub async fn run(dir: impl Into<PathBuf>) -> Result<()> {
        if let Some(agent) = Agent::run(dir).await? {
            let name = arc!(agent.manifest.agent.name.clone());

            if !Self::contains(name.clone()).await {
                MANAGER.lock().await.agents.insert(name, arc!(agent));
            }
        }

        Ok(())
    }

    /// Stops the agent server
    pub async fn stop(name: Arc<String>) -> Result<()> {
        let _ = MANAGER.lock().await.agents.remove(&name);
        Ok(())
    }

    /// Restarts the all agents who needs to bee restarted
    pub async fn update() -> Result<()> {
        // stop outdated agents:
        for name in MANAGER
            .get()
            .await
            .agents
            .iter()
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>()
            .into_iter()
        {
            if let Some(agent) = MANAGER.get().await.agents.get(&name)
                && agent.check().await?
            {
                Self::stop(name).await?;
            }
        }

        // re-init the agents manager (for run a new agents):
        Self::init().await?;

        Ok(())
    }
}
