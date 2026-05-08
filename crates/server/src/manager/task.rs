use crate::prelude::*;

/// The task delegation tool schema
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct TaskTool {
    task_id: i64,
    agent_name: String,
    task_query: String,
    wait_for: Option<i64>,
}
