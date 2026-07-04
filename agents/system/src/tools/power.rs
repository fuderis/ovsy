use crate::prelude::*;
use system_utils::{PowerManager, power::PowerMode};

#[derive(Deserialize)]
pub struct PowerAction {
    timestamp: Option<DateTime<Utc>>,
    mode: PowerMode,
}

/// API: Handles the `power` tool
#[log(skip_all, fields(action))]
pub async fn handle_schedule_power(tx: Sender<Bytes>, action: PowerAction) -> Result<()> {
    let local = action
        .timestamp
        .map(|utc| {
            utc.with_timezone(&Local)
                .format("%A, %I:%M:%S %p (%:z), %B %d, %Y")
                .to_string()
        })
        .unwrap_or("now".into());

    match PowerManager::schedule(action.mode, action.timestamp).await {
        Ok(_) => {
            let msg = str!(
                "Scheduled power action: {mode}. Execution time: {local}.",
                mode = action.mode
            );

            info!("{msg}");
            tx.send(Chunk::answer(msg)).await
        }

        Err(e) => Err(str!("Power operation failed: {e}").into()),
    }
}

/// API: Handles the `power` tool
#[log(skip_all)]
pub async fn handle_cancel_power(tx: Sender<Bytes>) -> Result<()> {
    let msg = match PowerManager::cancel().await {
        Some(mode) => str!("Scheduled power action canceled. Canceled action: {mode}."),
        None => str!("There is no scheduled power action."),
    };

    info!("{msg}");
    tx.send(Chunk::answer(msg)).await
}

/// API: Handles the `power` tool
#[log(skip_all)]
pub async fn handle_power_status(tx: Sender<Bytes>) -> Result<()> {
    let msg = match PowerManager::status().await {
        Some(task) => {
            str!(
                "Scheduled {mode}. Execution time: {local}",
                mode = task.mode,
                local = task
                    .execute_at
                    .with_timezone(&Local)
                    .format("%A, %I:%M:%S %p (%:z), %B %d, %Y"),
            )
        }

        None => str!("No power action is currently scheduled."),
    };

    info!("{msg}");
    tx.send(Chunk::answer(msg)).await
}
