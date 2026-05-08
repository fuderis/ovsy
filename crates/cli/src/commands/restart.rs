use crate::prelude::*;
use tokio::time;

/// Handles the `restart` command
pub async fn handle(restart_lms: bool) -> Result<()> {
    // stop server:
    super::stop::handle(restart_lms).await?;
    time::sleep(time::Duration::from_millis(800)).await;

    // starting away:
    super::start::handle(restart_lms).await?;

    Ok(())
}
