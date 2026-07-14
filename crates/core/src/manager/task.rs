use super::Tasks;
use crate::prelude::*;
use anylm::Content;
use ovsy_share::{AgentTask, Event};

/// The agent tasks handle
#[derive(Clone)]
pub struct Task {
    pub task_info: Arc<AgentTask>,
    pub tasks: Arc<Mutex<Tasks>>,
    pub tx: Sender<Bytes>,
}

impl Task {
    /// Returns true if this task is last
    pub async fn is_last(&self) -> bool {
        let lock = self.tasks.lock().await;
        lock.pending.is_empty() && lock.working.len() <= 1
        // <= 1 because current task is still listed in the working list before finish
    }

    /// Finishes the agent handling
    pub async fn finish(&self, agent_messages: Vec<Content>) {
        let mut lock = self.tasks.lock().await;

        lock.working.remove(&self.task_info.task_id);
        lock.finished.insert(self.task_info.task_id);

        // save results to ram:
        if lock.is_result_needed(self.task_info.task_id) {
            lock.results.insert(self.task_info.task_id, agent_messages);
        }

        // check pending tasks:
        if !lock.pending.is_empty() {
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

            for task in ready_to_run {
                let tx = self.tx.clone();
                let tasks = self.tasks.clone();

                tokio::spawn(async move {
                    crate::handlers::query::handle_task(task.task_id, tx, tasks).await;
                });
            }
        }
    }

    /// Finishes the all agent tasks
    pub async fn finish_all(&self) {
        let mut lock = self.tasks.lock().await;
        let working_tasks: Vec<_> = lock.working.drain().map(|(_, task)| task).collect();
        drop(lock);

        for task in working_tasks {
            task.abort();
        }
    }

    /// Finishes the entire task chain
    pub async fn finish_branch(&self) {
        let mut lock = self.tasks.lock().await;
        let mut to_abort = vec![];

        if let Some(child) = lock.working.remove(&self.task_info.task_id) {
            to_abort.push(child);
        };

        let mut to_remove = vec![self.task_info.task_id];
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
                        Event::error(str!("Cancelled: dependency task {} failed", task.task_id))
                            .task_info(&self.task_info),
                    );
                    to_remove.push(id);
                }
            }
            i += 1;
        }

        drop(lock);

        for task in to_abort {
            task.abort();
        }
    }

    /// Returns parent tasks context
    pub async fn context(&self) -> Vec<Vec<Content>> {
        let lock = self.tasks.lock().await;
        let mut results = vec![];

        for id in &self.task_info.wait_for {
            if let Some(res) = lock.results.get(&id) {
                results.push(res.clone())
            }
        }

        results
    }
}
