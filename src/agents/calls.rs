use crate::prelude::*;

/// The AI-agent query
#[derive(Deserialize)]
pub struct AgentTask {
    pub name: String,
    pub query: String,
}

/// The delegated user query
#[derive(Deserialize)]
pub struct DelegatedTasks {
    pub tasks: Option<Vec<AgentTask>>,
    pub say: Option<String>,
}

/// The AI-agent query
#[derive(Deserialize)]
pub struct AgentAction {
    pub name: String,
    pub data: JsonValue,
}
