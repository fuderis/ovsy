use crate::prelude::*;

/// The AI-agent query
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentTask {
    pub name: String,
    pub query: String,
    pub keys: Option<HashSet<String>>,
}

/// The delegated user query
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DelegatedTasks {
    pub tasks: Option<Vec<Vec<AgentTask>>>,
}

impl DelegatedTasks {
    /// Creates delegated tasks from cached agent
    pub fn from_cached_agent(name: impl Into<String>, query: impl Into<String>) -> Self {
        Self {
            tasks: Some(vec![vec![AgentTask {
                name: name.into(),
                query: query.into(),
                keys: None,
            }]]),
        }
    }
}

/// The AI-agent query
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentAction {
    pub name: String,
    pub data: JsonValue,
}

/// The summarize response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SummaryResults {
    pub answer: String,
    pub context: String,
}
