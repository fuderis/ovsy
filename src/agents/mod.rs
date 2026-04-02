pub mod agent;
pub use agent::Agent;
pub mod response;
pub use response::{AgentAction, AgentTask, FinalResponse};

use crate::prelude::*;
use anylm::Tool;
use tokio::{fs, time};

/// The agent instances
static AGENTS: State<Agents> = State::new();

/// The tools manager
#[derive(Default, Clone, Debug)]
pub struct Agents {
    agents: HashMap<String, Arc<Agent>>,
}

impl Agents {
    /// Returns true if tool name exists
    pub async fn has(name: &str) -> bool {
        AGENTS.get().await.agents.contains_key(name)
    }

    /// Returns agents map
    pub async fn get_all() -> Vec<Arc<Agent>> {
        AGENTS
            .get()
            .await
            .agents
            .iter()
            .map(|(_, a)| a.clone())
            .collect()
    }

    /// Returns a tool instance by name
    pub async fn get(name: &str) -> Option<Arc<Agent>> {
        AGENTS.get().await.agents.get(name).cloned()
    }

    /// Adds a tool to list
    pub async fn add(agent: Agent) {
        AGENTS
            .lock()
            .await
            .agents
            .insert(agent.manifest.agent.name.clone(), Arc::new(agent));
    }

    /// Stops tool & removes a from list
    pub async fn stop(name: &str) -> Result<bool> {
        let tool = AGENTS.lock().await.agents.remove(name);
        if let Some(tool) = tool {
            tool.stop().await?;
            return Ok(true);
        }
        Ok(false)
    }

    /// Returns agents list
    pub async fn list() -> Vec<String> {
        let mut list = vec![];

        for (name, agent) in AGENTS.get().await.agents.iter() {
            list.push(fmt!(
                "* {name} - {descr}",
                descr = &agent.manifest.agent.description,
            ));
        }

        list
    }

    /// Returns an agent by name
    async fn get_agent(name: &str) -> Option<Arc<Agent>> {
        AGENTS.get().await.agents.get(name).cloned()
    }

    /// Returns an agent use case examples
    pub async fn exmpls(name: &str) -> Vec<String> {
        Self::get_agent(name).await.unwrap().examples.clone()
    }

    /// Returns an agent AI tools (actions)
    pub async fn tools(name: &str) -> Vec<Tool> {
        Self::get_agent(name).await.unwrap().tools.clone()
    }

    /// Periodically manages all tools
    pub fn manage() {
        // run tool logs tracing:
        let timeout = {
            let cfg = &Settings::get().agents;
            cfg.trace_interval
        };
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_millis(timeout));

            loop {
                interval.tick().await;

                for (name, agent) in AGENTS.get().await.agents.iter() {
                    if let Some(trace) = &agent.trace
                        && let Some(lines) = trace.check().await
                    {
                        for line in lines {
                            println!("TRACE ({}): {line}", name.to_uppercase());
                        }
                    }
                }
            }
        });

        // run tools check:
        let (timeout, autocheck) = {
            let cfg = &Settings::get().agents;
            (cfg.check_interval, cfg.autocheck)
        };
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_millis(timeout));

            loop {
                interval.tick().await;
                let mut checked = HashSet::new();

                // check & restart existing agents:
                {
                    for (name, agent) in AGENTS.get().await.agents.iter() {
                        checked.insert(agent.dir.clone());

                        if let Err(e) = agent.check().await {
                            if let Some(e) = e.downcast_ref::<std::io::Error>()
                                && e.raw_os_error() == Some(32)
                            {
                                continue;
                            }

                            warn!("Failed to check '{name}' agent: {e}");
                        }
                    }
                }

                // scan directories for a new agents:
                for scan_dir in Settings::get().agents.scan_dirs.iter() {
                    let scan_dir: &PathBuf = scan_dir;

                    if let Ok(mut entries) = fs::read_dir(&scan_dir).await {
                        while let Ok(Some(entry)) = entries.next_entry().await {
                            let dir = entry.path();
                            if !dir.is_dir() || checked.contains(&dir) {
                                continue;
                            }
                            if let Err(e) = Agent::run(&dir).await {
                                trace!("Skipped agent dir '{}': {e}", dir.display());
                            }
                        }
                    }
                }

                // stop manage if autocheck disabled:
                if !autocheck {
                    break;
                }
            }
        });
    }
}
