use crate::prelude::*;
use anylm::ApiKind;
use colored::*;
use std::io::{self, Write};
use tokio::process::Command;

/// Handles the `stop` command
pub async fn stop(full: bool) -> Result<()> {
    let port = Settings::get().server.port;
    let red = Color::Red;

    println!("{} {}", "🛑".color(red), "Shutting down...".bold());

    print!(
        " {} Stopping Ovsy Server (port {})... ",
        "🔌".color(red),
        port
    );
    io::stdout().flush().ok();

    #[cfg(unix)]
    {
        let _ = Command::new("sh")
            .args(["-c", &format!("fuser -k {}/tcp", port)])
            .output()
            .await;
    }
    #[cfg(windows)]
    {
        let cmd = format!(
            "for /f \"tokens=5\" %a in ('netstat -aon ^| findstr \":{}\"') do taskkill /f /pid %a",
            port
        );
        let _ = Command::new("cmd").args(["/C", &cmd]).output().await;
    }
    println!("{}", "Stopped".red());

    let ai_conf = &Settings::get().assistant;
    if full
        && (ai_conf.completions.kind == ApiKind::LmStudio
            || ai_conf.embeddings.kind == ApiKind::LmStudio)
    {
        print!(" {} Unloading all models from VRAM... ", "🧹".color(red));
        io::stdout().flush().ok();

        // unload all LM Studio models:
        let _ = Command::new("lms").args(["unload", "--all"]).output().await;
        println!("{}", "Cleared".red());

        // stop LM Studio server:
        print!(" {} Shutting down LM Studio server... ", "💤".color(red));
        io::stdout().flush().ok();

        Command::new("lms")
            .args(["server", "stop"])
            .status()
            .await
            .ok();
        println!("{}", "Offline".red());
    }

    println!(
        " {}\n",
        "All processes terminated."
            .italic()
            .color(Color::AnsiColor(247))
    );
    Ok(())
}
