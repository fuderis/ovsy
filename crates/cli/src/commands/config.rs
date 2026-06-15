use crate::prelude::*;
use colored::*;
use tokio::process::Command;

/// API: Handles the `config` command
pub async fn handle_config() -> Result<()> {
    let path = Settings::path();

    println!("Opening config: {}", path.display().to_string().white());

    #[cfg(target_os = "linux")]
    let opener = "xdg-open";

    #[cfg(target_os = "macos")]
    let opener = "open";

    #[cfg(target_os = "windows")]
    let opener = "explorer";

    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    Command::new(opener)
        .arg(path)
        .spawn()
        .map_err(|e| str!("Failed to open config: {e}"))?;

    Ok(())
}
