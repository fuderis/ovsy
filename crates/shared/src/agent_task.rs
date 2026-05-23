use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

/// The AI-agent task
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub agent_name: String,
    #[serde(default = "AgentTask::random_id")]
    pub task_id: i64,
    pub task_query: String,
    #[serde(default)]
    pub wait_for: HashSet<i64>,
    #[serde(default)]
    pub tool_call_id: String,
}

impl AgentTask {
    /// Generates the random task ID
    fn random_id() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i64
    }

    /// Sets the tool call id
    pub fn tool_id(mut self, id: impl Into<String>) -> Self {
        self.tool_call_id = id.into();
        self
    }

    /// Returns the agent task clone (without query)
    pub fn clone_minimal(&self) -> Self {
        Self {
            agent_name: self.agent_name.clone(),
            task_id: self.task_id,
            task_query: String::new(),
            wait_for: HashSet::new(),
            tool_call_id: self.tool_call_id.clone(),
        }
    }
}
