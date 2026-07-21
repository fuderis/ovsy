use super::*;
use crate::prelude::*;

use ovsy_share::{AgentMetadata, StatusData};
use tokio::process::Command;

/// API: Handles the server refreshing (hot-reload)
pub async fn handle_refresh() -> Result<()> {
    let port = Settings::get().server.port;
    let client = Client::tcp();

    section("Refreshing Server");

    let res = client
        .get(&str!("http://127.0.0.1:{port}/refresh"))
        .send()
        .await;

    match res {
        Ok(response) => {
            info("Status", &str!("Online (port {port})").green().to_string());

            let data: StatusData = response
                .json()
                .await
                .map_err(|e| str!("Failed to parse response: {e}"))?;

            match data {
                StatusData::Success { .. } => {
                    success("Settings synchronized.");
                }

                StatusData::Error { error: err_msg } => {
                    error(err_msg.into());
                }
            }
        }

        Err(_) => {
            info("Server status", &"Offline".red().to_string());
            warn("Server is not responding. Check if it's running.");
            return Err(str!("Server is offline").into());
        }
    }

    println!();
    Ok(())
}

/// API: Handles the server status checking
pub async fn handle_status() -> Result<()> {
    let port = Settings::get().server.port;
    let client = Client::tcp();

    section("Checking Server");

    // checking Ovsy server:
    let res = client
        .get(&str!("http://127.0.0.1:{port}/status"))
        .send()
        .await;

    match res {
        Ok(response) => {
            info("Status", &str!("Online (port {port})").green().to_string());

            // parsing agents list:
            let data: StatusData = response
                .json()
                .await
                .map_err(|e| str!("Failed to parse response: {e}"))?;

            match data {
                StatusData::Success { agents } => {
                    section("Loaded Agents");

                    if agents.is_empty() {
                        warn("No agents loaded");
                    } else {
                        for AgentMetadata {
                            name, description, ..
                        } in agents
                        {
                            info(&name, &description.trim());
                        }
                    }
                }

                StatusData::Error { error: err_msg } => {
                    error(format!("Server error: {err_msg}").into());
                }
            }
        }
        Err(_) => {
            info("Status", &"Offline".red().to_string());
        }
    }

    section("Checking LMS Server");

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

    if lms_running {
        info(
            "Status",
            &str!("Online (port {lms_port})").green().to_string(),
        );

        let mut in_models_block = false;
        let mut found_any = false;

        for line in lms_raw.lines() {
            let line = line.trim();
            if line.contains("Loaded Models") {
                in_models_block = true;
                continue;
            }

            if in_models_block && line.starts_with('·') {
                if !found_any {
                    println!();
                    info("Loaded Models", "");
                }

                found_any = true;
                let model_info = line.trim_start_matches('·').trim();
                info("", &model_info.dimmed().to_string());
            }
        }
        if !found_any && in_models_block {
            warn("No models currently loaded in LMS");
        }
    } else {
        info("Status", &"Offline".red().to_string());
    }

    println!();
    Ok(())
}

/// API: Opens the config in the default editor
pub async fn handle_config() -> Result<()> {
    let path = Settings::path();

    section("Configuration");
    info("Path", &path.display().to_string().white().to_string());

    #[cfg(target_os = "linux")]
    let opener = "xdg-open";

    #[cfg(target_os = "macos")]
    let opener = "open";

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    match Command::new(opener).arg(&path).spawn() {
        Ok(_) => success("Config file opened in default editor."),
        Err(e) => error(str!("Failed to open config: {e}").into()),
    }

    println!();
    Ok(())
}
