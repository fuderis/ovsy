use crate::prelude::*;
use colored::*;
use ovsy_share::{AgentMetadata, StatusData};
use std::io::{self, Write};
use tokio::process::Command;

/// API: Handles the server refreshing (hot-reload)
pub async fn handle_refresh() -> Result<()> {
    let dim = Color::AnsiColor(247);
    let port = Settings::get().server.port;

    print!("{} ", "Updating Ovsy server...".bold());
    io::stdout().flush().ok();

    let client = Client::tcp();
    let res = client
        .post(&str!("http://127.0.0.1:{port}/refresh"))
        .send()
        .await;

    match res {
        Ok(response) => {
            println!("{}", str!("Online (port {port})").green());

            let data: StatusData = response
                .json()
                .await
                .map_err(|e| str!(str!("Failed to parse response: {e}")))?;

            match data {
                StatusData::Success { agents } => {
                    if agents.is_empty() {
                        println!("   {}", "No agents loaded".yellow().dimmed());
                    } else {
                        for AgentMetadata { name, .. } in agents {
                            println!(" • {}", name.dimmed());
                        }
                    }

                    println!("\n{}", "Settings synchronized.".bright_white());
                }
                StatusData::Error { error } => {
                    println!("   {} {}", "Error:".red().bold(), error.white());
                }
            }
        }
        Err(_) => {
            println!("{}", "Offline".red());
            return Err(str!("Server is not responding. Check if it's running.").into());
        }
    }

    super::underline();
    println!("{}\n", "Environment synchronized.".italic().color(dim));

    Ok(())
}

/// API: Handles the server status checking
pub async fn handle_status() -> Result<()> {
    let port = Settings::get().server.port;
    let client = Client::tcp();

    // checking Ovsy server:
    let res = client
        .get(&str!("http://127.0.0.1:{port}/status"))
        .send()
        .await;

    match res {
        Ok(response) => {
            println!(
                "Checking Ovsy server... {}",
                str!("Online ({port})").green()
            );

            // parsing agents list:
            if let Ok(data) = response.json::<StatusData>().await {
                match data {
                    StatusData::Success { agents } => {
                        if agents.is_empty() {
                            println!("   {}", "No agents loaded".yellow().dimmed());
                        } else {
                            for AgentMetadata { name, .. } in agents {
                                println!(" • {}", name.dimmed());
                            }
                        }
                    }
                    StatusData::Error { error } => {
                        println!("   {} {}", "Error:".red(), error.dimmed());
                    }
                }
            }
        }
        Err(_) => {
            println!("Checking Ovsy server... {}", "Offline".red());
        }
    }

    // checking LMS server:
    let lms_out = Command::new("lms").args(["status"]).output().await;
    let lms_raw = match lms_out {
        Ok(out) => String::from_utf8_lossy(&out.stdout).to_string(),
        _ => String::new(),
    };

    let lms_running = lms_raw.contains("ON");
    let lms_port = lms_raw
        .lines()
        .find(|l| l.contains("port:"))
        .and_then(|l| l.split("port:").last())
        .map(|p| p.trim_matches(|c: char| !c.is_numeric()))
        .unwrap_or("unknown");

    let lms_display = if lms_running {
        str!("Online ({lms_port})").green()
    } else {
        "Offline".red()
    };

    println!("Checking LMS server... {}", lms_display);

    if lms_running {
        let mut in_models_block = false;
        let mut found_any = false;

        for line in lms_raw.lines() {
            let line = line.trim();
            if line.contains("Loaded Models") {
                in_models_block = true;
                continue;
            }
            if in_models_block && line.starts_with('·') {
                found_any = true;
                let model_info = line.trim_start_matches('·').trim();
                println!(" ∟ {}", model_info.dimmed());
            }
        }
        if !found_any && in_models_block {
            println!("   {}", "None".yellow().dimmed());
        }
    }

    Ok(())
}

/// API: Opens the config on the default editor
pub async fn handle_config() -> Result<()> {
    let path = Settings::path();

    println!("Opening config: {}", path.display().to_string().white());

    #[cfg(target_os = "linux")]
    let opener = "xdg-open";

    #[cfg(target_os = "macos")]
    let opener = "open";

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    Command::new(opener)
        .arg(path)
        .spawn()
        .map_err(|e| str!("Failed to open config: {e}"))?;

    Ok(())
}
