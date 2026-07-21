use super::*;
use crate::prelude::*;

use anylm::ApiKind;
use std::{net::TcpListener, process::Stdio, time::Duration};
use tokio::{process::Command, time::sleep};

/// API: Handles the server launching
pub async fn handle_start(start_lms: bool) -> Result<()> {
    section("Starting Services");

    // 1. Ovsy Server
    let port = Settings::get().server.port;
    let is_port_free = TcpListener::bind(str!("127.0.0.1:{port}")).is_ok();

    if is_port_free {
        Command::new(path!("$"))
            .arg("serve")
            .current_dir(path!("$/"))
            .kill_on_drop(false)
            .spawn()?;
        info("Ovsy Server", &"Online".green().to_string());
    } else {
        warn(&format!("Ovsy Server: Port {port} is already in use"));
    }

    // 2. LMS Server
    let ai_conf = &Settings::get().assistant;
    if start_lms
        && (ai_conf.completions.kind == ApiKind::LmStudio
            || ai_conf.embeddings.kind == ApiKind::LmStudio)
    {
        let is_running = match Command::new("lms").args(["status"]).output().await {
            Ok(out) => String::from_utf8_lossy(&out.stdout).contains("ON"),
            _ => false,
        };

        if !is_running {
            match Command::new("lms")
                .args(["server", "start"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(_child) => {
                    let mut is_ok = false;

                    // 100 tries * 100 ms = 10 seconds to start:
                    for _ in 0..100 {
                        sleep(Duration::from_millis(100)).await;

                        let status_check = Command::new("lms").args(["status"]).output().await;

                        if let Ok(out) = status_check {
                            if String::from_utf8_lossy(&out.stdout).contains("ON") {
                                is_ok = true;
                                break;
                            }
                        }
                    }

                    if is_ok {
                        info("LMS Server", &"Online".green().to_string());
                    } else {
                        info("LMS Server", &"Failed to start".red().to_string());
                    }
                }
                Err(e) => {
                    error(format!("Failed to spawn LMS process: {e}").into());
                    info("LMS Server", &"Failed".red().to_string());
                }
            }
        } else {
            info("LMS Server", &"Online".green().to_string());
        }

        // 3. Loading Models
        let loaded_models = Command::new("lms")
            .args(["ps"])
            .output()
            .await
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

        let mut models = vec![];

        if ai_conf.completions.kind.is_lmstudio() {
            models.push(&ai_conf.completions.model);
        }
        if ai_conf.embeddings.kind.is_lmstudio() && Settings::get().cache.enable {
            models.push(&ai_conf.embeddings.model);
        }
        if ai_conf.compression.kind.is_lmstudio()
            && ai_conf.compression.model != ai_conf.completions.model
        {
            models.push(&ai_conf.compression.model);
        }

        for model in models {
            if !model.is_empty() {
                if !loaded_models.contains(model) {
                    let status = Command::new("lms")
                        .args(["load", model])
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .stdin(Stdio::null())
                        .status()
                        .await;

                    if status.map_or(false, |s| s.success()) {
                        item("", &str!("{model} (Loaded)").green().to_string());
                    } else {
                        warn(&format!("Failed to load model: {model}"));
                    }
                } else {
                    item("", &str!("{model} (Already loaded)").dim().to_string());
                }
            }
        }
    }

    println!();
    success("Ready for requests!");
    println!();

    Ok(())
}

/// API: Handles the server shutdown
pub async fn handle_stop(stop_lms: bool) -> Result<()> {
    section("Stopping Services");

    let port = Settings::get().server.port;

    // 1. Stop Ovsy server
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
    info("Ovsy Server", &"Offline".red().to_string());

    // 2. Stop LMS server
    let ai_conf = &Settings::get().assistant;
    if stop_lms
        && (ai_conf.completions.kind == ApiKind::LmStudio
            || ai_conf.embeddings.kind == ApiKind::LmStudio)
    {
        // Unload models first
        let _ = Command::new("lms").args(["unload", "--all"]).output().await;
        info("LMS Models", &"Unloaded".red().to_string());

        Command::new("lms")
            .args(["server", "stop"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .ok();
        info("LMS Server", &"Offline".red().to_string());
    }

    println!();
    success("Processes terminated.");
    println!();

    Ok(())
}

/// API: Handles the server restarting
pub async fn handle_restart(restart_lms: bool) -> Result<()> {
    handle_stop(restart_lms).await?;
    sleep(Duration::from_millis(800)).await;
    handle_start(restart_lms).await?;

    Ok(())
}
