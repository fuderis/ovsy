use crate::prelude::*;

/// The AI-agent query
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentTask {
    #[serde(default)]
    pub name: String,
    pub id: u32,
    pub query: String,
    pub wait_for: Option<u32>,
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
