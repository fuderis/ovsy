use crate::{power::*, prelude::*};

/// API: Handles the `power` tool
pub async fn handle(tx: Arc<StreamSender<Bytes>>, data: JsonValue) -> Result<()> {
    let action: PowerAction = json::from_value(data)?;

    match Power::execute(action.clone()).await {
        Ok(status) => {
            use PowerStatus::*;

            let fmt_local = |utc_time: DateTime<Utc>| {
                utc_time
                    .with_timezone(&Local)
                    .format("%A, %B %d, %Y %I:%M:%S %p UTC%:z")
                    .to_string()
            };

            let msg = match status {
                Executed => str!("System power action executed successfully."),
                Deferred {
                    mode,
                    target_time,
                    remaining_secs,
                } => {
                    str!(
                        "Power action {mode} is scheduled at {} (in {remaining_secs}s).",
                        fmt_local(target_time)
                    )
                }
                Canceled { mode } => {
                    str!("Power action {mode} is canceled!")
                }
                ActiveTask {
                    mode,
                    target_time,
                    remaining_secs,
                } => {
                    str!(
                        "Power action {mode} is scheduled for {} (Remaining: {remaining_secs}s)",
                        fmt_local(target_time)
                    )
                }
                NoActiveTask => match action.mode {
                    PowerMode::Cancel => str!("Nothing to cancel."),
                    _ => str!("No power actions scheduled."),
                },
            };

            info!("{msg}");
            tx.send(Chunk::answer(msg))?;
            Ok(())
        }
        Err(e) => {
            error!("Power operation failed: {e}");
            Err(str!("Power operation failed: {e}").into())
        }
    }
}
