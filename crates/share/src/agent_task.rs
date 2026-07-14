use crate::SessionId;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

/// The AI-agent task
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    #[serde(default)]
    pub session_id: SessionId,
    pub agent_name: String,
    pub agent_skills: Vec<String>,
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

    /// Sets the session id
    pub fn sess_id(mut self, id: SessionId) -> Self {
        self.session_id = id;
        self
    }

    /// Sets the tool call id
    pub fn tool_id(mut self, id: impl Into<String>) -> Self {
        self.tool_call_id = id.into();
        self
    }
}
