use crate::prelude::*;
use tokio::time;

/// API: Handles the server restarting
pub async fn handle_restart(restart_lms: bool) -> Result<()> {
    // stop server:
    super::handle_stop(restart_lms).await?;
    time::sleep(time::Duration::from_millis(800)).await;

    // starting away:
    super::handle_start(restart_lms).await?;

    Ok(())
}
