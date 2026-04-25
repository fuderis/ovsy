use crate::{UNDERLINE_COUNT, prelude::*};
use anylm::ApiKind;
use colored::*;
use std::{
    io::{self, Write},
    process::Stdio,
};
use tokio::process::Command;

/// Handles the `start` command
pub async fn handle() -> Result<()> {
    let exe = std::env::consts::EXE_SUFFIX;
    let server_path = app_data().join(format!("ovsy-server{exe}"));

    if !server_path.exists() {
        return Err(str!("Server binary missing. Run 'ovsy build' first.").into());
    }

    let ai_conf = &Settings::get().assistant;
    if ai_conf.completions.kind == ApiKind::LmStudio || ai_conf.embeddings.kind == ApiKind::LmStudio
    {
        print!("📡 Checking LM Studio server... ");
        io::stdout().flush().ok();

        let server_status = Command::new("lms").args(["status"]).output().await;
        let is_running = match server_status {
            Ok(out) => String::from_utf8_lossy(&out.stdout).contains("ON"),
            _ => false,
        };

        if !is_running {
            println!("{}", "Offline. Starting...".yellow());
            Command::new("lms").args(["server", "start"]).spawn().ok();
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        } else {
            println!("{}", "Online".green());
        }

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
            print!(" • Loading model {}... ", model.dimmed());
            io::stdout().flush().ok();

            if !model.is_empty() {
                if !loaded_models.contains(model) {
                    Command::new("lms")
                        .args(["load", model])
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .stdin(Stdio::null())
                        .status()
                        .await
                        .ok();
                }
                println!("{}", "Loaded".green());
            }
        }
    }

    let port = Settings::get().server.port;
    print!("\n🚀 Starting Ovsy server... ");
    io::stdout().flush().ok();

    // check for for busy:
    let is_port_free = std::net::TcpListener::bind(format!("127.0.0.1:{port}")).is_ok();
    if is_port_free {
        Command::new(server_path)
            .current_dir(app_data())
            .kill_on_drop(false)
            .spawn()?;
    }
    println!("{}", "Online".green());

    println!(
        "{}",
        "─".repeat(UNDERLINE_COUNT).color(Color::AnsiColor(240))
    );
    println!("{}\n", "System is ready for requests.".italic().dimmed());

    Ok(())
}
