use crate::prelude::*;
use super::Tool;
use tokio::time;

/// The tools instances
static TOOLS: State<Tools> = State::new();

/// The tools manager
#[derive(Default, Clone, Debug)]
pub struct Tools {
    tools: HashMap<String, Tool>,
}

impl Tools {
    /// Returns true if tool name exists
    pub async fn has(name: &str) -> bool {
        TOOLS.get().await.tools.contains_key(name)
    }
    
    /// Returns a tool instance by name
    pub async fn get(name: &str) -> Option<Tool> {
        TOOLS.get().await.tools.get(name).map(|tool| tool.clone())
    }

    /// Adds a tool to list
    pub async fn add(tool: Tool) {
        TOOLS.lock().await.tools.insert(tool.manifest.tool.name.clone(), tool);
    }

    /// Stops tool & removes a from list
    pub async fn stop(name: &str) -> Result<bool> {
        if let Some(tool) = TOOLS.lock().await.tools.remove(name) {
            tool.stop().await?;
            return Ok(true);
        }
        Ok(false)
    }

    /// Returns action docs list
    pub async fn docs() -> Vec<String> {
        let mut docs = vec![];
        for (_, tool) in TOOLS.get().await.tools.iter() {
            docs.extend(tool.docs.clone());
        }
        docs
    }

    /// Periodically manages all tools
    pub fn manage(timeout: u64) {        
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_millis(timeout));
            
            loop {
                interval.tick().await;
                let mut checked = vec![];
                
                // check & restart existing tools:
                {
                    for (name, tool) in TOOLS.get().await.tools.iter() {
                        checked.push(tool.dir.clone());
                        
                        if let Err(e) = tool.check().await {
                            if let Some(e) = e.downcast_ref::<std::io::Error>() {
                                if e.raw_os_error() == Some(32) {
                                    continue;
                                }
                            }
                            
                            warn!("Fail with check tool '{name}': {e}");
                        }
                    }
                }

                // scan tools directory for new tools:
                for dir in Settings::get().tools.dirs.iter() {
                    let dir: &PathBuf = dir;
                    if dir.is_dir() && !checked.contains(&dir) {
                        if let Err(e) = Tool::run(&dir).await {
                            trace!("Skipped tool dir '{}': {e}", dir.display());
                        }
                    }
                }
            }
        });
    }
}

