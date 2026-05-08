use crate::prelude::*;
use anylm::ApiKind;
use colored::*;
use std::{
    io::{self, Write},
    process::Stdio,
};
use tokio::process::Command;

/// Handles the `stop` command
pub async fn handle(stop_lms: bool) -> Result<()> {
    let port = Settings::get().server.port;

    // stop Ovsy server:
    print!("Shutting down Ovsy Server... ");
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
    println!("{}", "Offline".red());

    // stop LMS server:
    print!("Shutting down LMS server... ");
    io::stdout().flush().ok();

    Command::new("lms")
        .args(["server", "stop"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .ok();
    println!("{}", "Offline".red());

    // unload LMS models:
    let ai_conf = &Settings::get().assistant;
    if stop_lms
        && (ai_conf.completions.kind == ApiKind::LmStudio
            || ai_conf.embeddings.kind == ApiKind::LmStudio)
    {
        print!(" ∟ Unloading models... ");
        io::stdout().flush().ok();

        // unload all LM Studio models:
        let _ = Command::new("lms").args(["unload", "--all"]).output().await;
        println!("{}", "Unloaded".red());
    }

    super::underline();
    println!(
        " {}\n",
        "Processes terminated."
            .italic()
            .color(Color::AnsiColor(247))
    );
    Ok(())
}
