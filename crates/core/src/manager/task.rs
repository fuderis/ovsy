use super::Tasks;
use crate::prelude::*;
use anylm::Content;
use ovsy_share::{AgentTask, Chunk};

/// The agent tasks handle
#[derive(Clone)]
pub struct Task {
    pub task: Arc<AgentTask>,
    tasks: Arc<Mutex<Tasks>>,
    tx: Sender,
}

impl Task {
    /// Handles the agent task or pendings it
    #[async_recursion]
    pub async fn handle(tx: Sender, task_id: i64, tasks: Arc<Mutex<Tasks>>) {
        let mut lock = tasks.lock().await;
        let Some(task) = lock.pending.remove(&task_id) else {
            return;
        };

        let tx = tx.clone();
        let tasks = tasks.clone();

        // handle agent task:
        let messages = lock.messages.clone();
        let child = tokio::spawn(async move {
            let handle = Self {
                task: arc!(task),
                tasks: tasks.clone(),
                tx: tx.clone(),
            };

            let session_id = handle.task.session_id;
            let session = tasks.lock().await.session.clone();
            if let Err(e) = crate::handlers::handle::handle_agent(
                session_id,
                session,
                messages,
                handle.task.agent_name.clone(),
                tx,
                handle.clone(),
            )
            .await
            {
                error!("[handle_agent{{sid={session_id}}}] {e}");
                handle.tx.send(Chunk::error(str!("{e}"))).ok();
                handle.finish_branch().await;
            }
        });

        lock.working.insert(task_id, arc!(child));
    }

    /// Returns true if this task is last
    pub async fn is_last(&self) -> bool {
        let lock = self.tasks.lock().await;
        lock.pending.is_empty() && lock.working.len() <= 1
        // <= 1 because current task is still listed in the working list before finish
    }

    /// Finishes the agent handling
    pub async fn finish(&self, agent_messages: Vec<Content>) {
        let mut lock = self.tasks.lock().await;

        lock.working.remove(&self.task.task_id);
        lock.finished.insert(self.task.task_id);

        // ОПТИМИЗАЦИЯ ПАМЯТИ: сохраняем Messages в RAM только если они нужны наследникам
        if lock.is_result_needed(self.task.task_id) {
            lock.results.insert(self.task.task_id, agent_messages);
        }

        // Если есть готовые к запуску зависимые задачи
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
                    Self::handle(tx, task.task_id, tasks).await;
                });
            }
        }
        // ФИНАЛ: Весь граф успешно выполнен! Записываем ВСЮ историю в БД одним махом
        else if lock.working.is_empty() {
            let session = lock.session.clone();
            let ctx_messages = lock.messages.clone();
            drop(lock);

            tokio::spawn(async move {
                let final_lock = ctx_messages.lock().await;
                if let Err(e) = session.write_messages(final_lock.messages.clone()).await {
                    error!("[Task::finish] Failed to atomic save final conversation: {e}");
                }
            });
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
            task.abort();
        }
    }

    /// Returns parent tasks context
    pub async fn context(&self) -> Vec<Vec<Content>> {
        let lock = self.tasks.lock().await;
        let mut results = vec![];

        for id in &self.task.wait_for {
            if let Some(res) = lock.results.get(&id) {
                results.push(res.clone())
            }
        }

        results
    }
}
