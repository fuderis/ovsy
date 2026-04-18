use crate::prelude::*;
use colored::*;
use tokio::time;

/// Handles the `restart` command
pub async fn restart(full: bool) -> Result<()> {
    let cyan = Color::Cyan;

    println!("{} {}", "🔄".color(cyan), "Restarting Ecosystem...".bold());

    // stop server:
    super::stop(full).await?;
    time::sleep(time::Duration::from_millis(800)).await;

    // starting away:
    super::start().await?;

    Ok(())
}
