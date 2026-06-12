use crate::prelude::*;
use anylm::{Content, Messages};
use ovsy_share::AgentTask;
use tokio::task::JoinHandle;

/// The agent tasks worflow
pub struct Tasks {
    pub working: HashMap<i64, Arc<JoinHandle<()>>>,
    pub pending: HashMap<i64, AgentTask>,
    pub finished: HashSet<i64>,
    pub results: HashMap<i64, Vec<Content>>,
    pub session: Session,
    pub messages: Arc<Mutex<Messages>>,
}

impl Tasks {
    /// Creates a new workflow
    pub fn new(session: Session, messages: Arc<Mutex<Messages>>) -> Arc<Mutex<Self>> {
        arc!(Mutex::new(Self {
            pending: HashMap::new(),
            working: HashMap::new(),
            finished: HashSet::new(),
            results: HashMap::new(),
            session,
            messages,
        }))
    }

    /// Returns true if task ready to start
    pub fn check(&self, task: &AgentTask) -> bool {
        task.wait_for.is_empty() || task.wait_for.iter().all(|id| self.finished.contains(id))
    }

    /// Returns true if task result is required
    pub fn is_result_needed(&self, task_id: i64) -> bool {
        self.pending.values().any(|t| t.wait_for.contains(&task_id))
    }
}
