use crate::{handlers::query, prelude::*};
use async_recursion::async_recursion;
use ovsy_shared::{AgentTask, Chunk};
use tokio::task::JoinHandle;

/// The agent tasks worflow
#[derive(Default)]
pub struct Workflow {
    working: HashMap<i64, Arc<Mutex<JoinHandle<()>>>>,
    pending: HashMap<i64, AgentTask>,
    finished: HashSet<i64>,
    results: HashMap<i64, Arc<String>>,
}

impl Workflow {
    /// Creates a new workflow
    pub fn new() -> Arc<Mutex<Self>> {
        arc_mutex!(Self::default())
    }

    /// Returns true if task ready to start
    pub fn check(&self, task: &AgentTask) -> bool {
        task.wait_for.is_empty() || task.wait_for.iter().all(|id| self.finished.contains(id))
    }
}

/// The agent tasks handle
#[derive(Clone)]
pub struct AgentHandle {
    pub task: Arc<AgentTask>,
    workflow: Arc<Mutex<Workflow>>,
    tx: Sender,
}

impl AgentHandle {
    /// Handles the all agent tasks
    pub async fn handle_all(tx: Sender, tasks: Vec<AgentTask>) {
        if tasks.is_empty() {
            return;
        }

        let workflow = Workflow::new();

        // collect tasks:
        let mut running = vec![];
        {
            let mut lock = workflow.lock().await;

            for task in tasks {
                if task.wait_for.is_empty() {
                    running.push(task.task_id);
                }

                lock.pending.insert(task.task_id, task);
            }
        };

        // running tasks:
        for id in running {
            Self::handle(tx.clone(), id, workflow.clone()).await;
        }
    }

    /// Handles the agent task or pendings it
    #[async_recursion]
    async fn handle(tx: Sender, task_id: i64, workflow: Arc<Mutex<Workflow>>) {
        let mut lock = workflow.lock().await;
        let task = if let Some(task) = lock.pending.remove(&task_id) {
            task
        } else {
            return;
        };

        let tx = tx.clone();
        let workflow = workflow.clone();

        // handle agent task:
        let child = tokio::spawn(async move {
            let handle = Self {
                task: arc!(task),
                workflow: workflow.clone(),
                tx: tx.clone(),
            };

            if let Err(e) = query::handle_agent(tx, handle.clone()).await {
                handle.tx.send(Chunk::error(str!("{e}"))).ok();
                handle.finish_branch().await;
            }
        });

        lock.working.insert(task_id, arc_mutex!(child));
    }

    /// Finishes the agent handling
    pub async fn finish(&self, full_text: String) {
        let mut lock = self.workflow.lock().await;

        // remove from active:
        lock.working.remove(&self.task.task_id);
        // mark as finished:
        lock.finished.insert(self.task.task_id);
        // write results to context:
        lock.results.insert(self.task.task_id, arc!(full_text));

        // check pending tasks:
        let mut ready_ids = vec![];
        for (id, task) in lock.pending.iter() {
            if lock.check(&task) {
                ready_ids.push(*id);
            }
        }
        let ready_to_run: Vec<AgentTask> = ready_ids
            .into_iter()
            .filter_map(|id| lock.pending.remove(&id))
            .collect();

        drop(lock);

        // start next tasks:
        for task in ready_to_run {
            let tx = self.tx.clone();
            let workflow = self.workflow.clone();

            tokio::spawn(async move {
                Self::handle(tx, task.task_id, workflow).await;
            });
        }

        self.tx.send(Chunk::finish()).ok();
    }

    /// Finishes the all agent tasks
    pub async fn finish_all(&self) {
        let mut lock = self.workflow.lock().await;
        let working_tasks: Vec<_> = lock.working.drain().map(|(_, task)| task).collect();
        drop(lock);

        for task in working_tasks {
            task.lock().await.abort();
        }
    }

    /// Finishes the entire task chain
    pub async fn finish_branch(&self) {
        let mut lock = self.workflow.lock().await;
        let mut to_abort = vec![];

        if let Some(child) = lock.working.remove(&self.task.task_id) {
            to_abort.push(child);
        };

        let mut to_remove = vec![self.task.task_id];
        let mut i = 0;

        while i < to_remove.len() {
            let current_id = to_remove[i];
            let mut dependents = vec![];

            for (id, task) in lock.pending.iter() {
                if task.wait_for.contains(&current_id) {
                    dependents.push(*id);
                }
            }

            for id in dependents {
                if let Some(task) = lock.pending.remove(&id) {
                    let _ = self.tx.send(
                        Chunk::error(str!("Cancelled: dependency task {} failed", task.task_id))
                            .task_info(self.task.clone_minimal()),
                    );
                    to_remove.push(id);
                }
            }
            i += 1;
        }

        drop(lock);

        for task in to_abort {
            task.lock().await.abort();
        }
    }

    /// Returns parent tasks context
    pub async fn context(&self) -> Vec<Arc<String>> {
        let lock = self.workflow.lock().await;
        let mut results = vec![];

        for id in &self.task.wait_for {
            if let Some(res) = lock.results.get(&id) {
                results.push(res.clone())
            }
        }

        results
    }
}
