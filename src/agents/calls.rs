use crate::prelude::*;

/// The AI-agent query
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentTask {
    pub name: String,
    pub query: String,
    pub keys: HashSet<String>,
}

/// The delegated user query
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DelegatedTasks {
    pub tasks: Option<Vec<AgentTask>>,
    pub say: Option<String>,
}

/// The AI-agent query
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentAction {
    pub name: String,
    pub data: JsonValue,
}
