pub mod agent;
pub use agent::Agent;

pub mod handle;
pub use handle::{AgentHandle, Workflow};

use crate::prelude::*;
use anylm::{Schema, Tool};
use ovsy_shared::AgentInfo;
use std::fmt::Write;
use tokio::task::JoinSet;

/// The agents manager state
pub static MANAGER: State<Manager> = State::new();

/// The agents manager
#[derive(Default, Debug, Clone)]
pub struct Manager {
    pub agents: HashMap<Arc<String>, Arc<Agent>>,
    pub agents_doc: Arc<String>,
    pub task_tool: Arc<Tool>,
}

impl Manager {
    /// Initializes & runs the agents management
    pub async fn init() -> Result<()> {
        let scan_dir = app_data().join("agents");

        // check scan dir:
        if !scan_dir.exists() {
            warn!("[MANAGER] Agents directory not found at: {scan_dir:?}");
            return Ok(());
        }

        let mut set = JoinSet::new();
        let mut reader = Dir::read(scan_dir).await?;

        info!("[MANAGER] Scanning for agents...");

        // read agents dirs:
        while let Some(entry) = reader.next_dir().await? {
            let path = entry.path().clone();

            // skip non-agents:
            if !path.join("Ovsy.toml").is_file() {
                continue;
            }

            // spawn agent running:
            set.spawn(async move { Self::run(path).await });
        }

        // check results:
        while let Some(task_res) = set.join_next().await {
            if let Err(e) = task_res {
                error!("[MANAGER] Agent startup task panicked: {e}");
            }
        }

        // gen task delegation tool:
        Self::gen_task_tool().await;

        Ok(())
    }

    /// Generates & sets the task schema
    pub async fn gen_task_tool() {
        let tool = Tool::new(
            "handle_agent",
            "Delegates a task to a specific AI agent for execution (do not invent non-existent agents).",
        )
        .required_property(
            "agent_name",
            Schema::string("The name of the agent to handle this task."),
        )
        .required_property(
            "task_id",
            Schema::integer("An unique identifier for the task (starting from 1, and should not be repeated)."),
        )
        .required_property(
            "task_query",
            Schema::string("The task query and data for the agent handling (describe the task in details)."),
        )
        .optional_property(
            "wait_for",
            Schema::array("Identifiers of tasks that must be completed before this one (when need the results of another tasks).")
                .items(Schema::integer("Identifier of task that must be completed before."))
        );

        MANAGER.lock().await.task_tool = arc!(tool);
    }

    /// Runs the AI agent server
    pub async fn run(dir: impl Into<PathBuf>) -> Result<()> {
        let path: PathBuf = dir.into();
        info!("[MANAGER] Starting agent from: {:?}", path);

        if let Some(agent) = Agent::run(path.clone()).await? {
            let name = arc!(agent.manifest.agent.name.clone());
            let mut lock = MANAGER.lock().await;

            if !lock.agents.contains_key(&name) {
                lock.agents.insert(name.clone(), arc!(agent));
                info!("[MANAGER] Agent [{name}] added to manager");
            } else {
                warn!("[MANAGER] Agent [{name}] is already running, skipping");
            }
        }

        Self::update_doc().await?;
        Ok(())
    }

    /// Stops the AI agent server
    pub async fn stop(name: Arc<String>) -> Result<()> {
        let mut lock = MANAGER.lock().await;
        if lock.agents.remove(&name).is_some() {
            info!("[MANAGER] Agent [{}] stopped and removed", name);
        } else {
            warn!("[MANAGER] Attempted to stop unknown agent: [{}]", name);
        }

        Self::update_doc().await?;
        Ok(())
    }

    /// Updates the AI agents list
    pub async fn update() -> Result<()> {
        info!("[MANAGER] Starting agents update cycle...");

        // collect the list of all the outdated agents:
        let mut to_restart = Vec::new();
        {
            let guard = MANAGER.get().await;
            for (name, agent) in &guard.agents {
                if agent.check().await? {
                    to_restart.push(name.clone());
                }
            }
        }

        // stop all the outdated agents:
        for name in to_restart {
            warn!("[MANAGER] Agent [{}] needs update, restarting...", name);
            Self::stop(name).await?;
        }

        Self::init().await?;
        info!("[MANAGER] Agents update cycle completed");
        Ok(())
    }

    /// Updates the agents list prompt part
    pub async fn update_doc() -> Result<()> {
        let guard = MANAGER.get().await;

        // gen message, if agents not found:
        if guard.agents.is_empty() {
            MANAGER.lock().await.agents_doc = arc!("No active agents available.".to_string());
            return Ok(());
        }

        // gen agents doc:
        let mut doc_builder = String::from("Available Agents:\n");
        for agent in guard.agents.values() {
            let _ = writeln!(
                doc_builder,
                "* Agent `{}`: {}",
                agent.manifest.agent.name, agent.manifest.agent.description
            );
        }

        MANAGER.lock().await.agents_doc = arc!(doc_builder);
        info!(
            "[MANAGER] Documentation updated: {} agents listed",
            guard.agents.len()
        );
        Ok(())
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

    /// Returns true if agent with this name is already on running
    pub async fn contains(name: &Arc<String>) -> bool {
        MANAGER.get().await.agents.contains_key(name)
    }

    /// Returns the agents list prompt part
    pub async fn agents_list_doc() -> Arc<String> {
        MANAGER.get().await.agents_doc.clone()
    }

    /// Returns the task delegation tool
    pub async fn task_tool() -> Tool {
        (*MANAGER.get().await.task_tool).clone()
    }

    /// Returns the agent system prompt
    pub async fn agent_prompt(name: &Arc<String>) -> Option<String> {
        MANAGER
            .get()
            .await
            .agents
            .get(name)
            .map(|agent| agent.manifest.agent.prompt.clone())
    }

    /// Returns the agent tools list
    pub async fn agent_tools(name: &Arc<String>) -> Option<Vec<Tool>> {
        MANAGER
            .get()
            .await
            .agents
            .get(name)
            .map(|agent| agent.manifest.tools.clone())
    }

    /// Returns the agent options (port, prompt, tools)
    pub async fn agent_options(name: &Arc<String>) -> Option<(u16, String, Vec<Tool>)> {
        MANAGER.get().await.agents.get(name).map(|agent| {
            (
                agent.port,
                agent.manifest.agent.prompt.clone(),
                agent.manifest.tools.clone(),
            )
        })
    }
}
