use crate::{UNDERLINE_COUNT, prelude::*};
use anylm::ApiKind;
use colored::*;
use std::io::{self, Write};
use tokio::process::Command;

/// Handles the `start` command
pub async fn start() -> Result<()> {
    let cyan = Color::Cyan;
    let dim = Color::AnsiColor(247);

    println!(
        "{} {}",
        "🚀".color(cyan),
        "Starting Ovsy Ecosystem...".bold()
    );
    println!(
        "{}",
        "─".repeat(UNDERLINE_COUNT).color(Color::AnsiColor(240))
    );

    let exe = std::env::consts::EXE_SUFFIX;
    let server_path = app_data().join(format!("ovsy-server{exe}"));

    if !server_path.exists() {
        return Err(str!("Server binary missing. Run 'ovsy build' first.").into());
    }

    let ai_conf = &Settings::get().assistant;
    if ai_conf.completions.kind == ApiKind::LmStudio || ai_conf.embeddings.kind == ApiKind::LmStudio
    {
        print!(" {} Checking LM Studio... ", "📡".color(cyan));
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

        for (kind, model) in [
            (&ai_conf.completions.kind, &ai_conf.completions.model),
            (&ai_conf.embeddings.kind, &ai_conf.embeddings.model),
        ] {
            if *kind != ApiKind::LmStudio {
                continue;
            }

            print!(
                " {} Loading model {}... ",
                "🧠".color(cyan),
                model.bright_white()
            );
            io::stdout().flush().ok();

            if !model.is_empty() {
                if !loaded_models.contains(model) {
                    Command::new("lms")
                        .args(["load", model])
                        .status()
                        .await
                        .ok();
                    // println!("{}", "Done".green());
                } else {
                    println!("{}", "Ready".green());
                }
            }
        }
    }

    let port = Settings::get().server.port;
    print!(" {} Starting Ovsy server... ", "⚡".color(cyan));
    io::stdout().flush().ok();

    // check for for busy:
    let is_port_free = std::net::TcpListener::bind(format!("127.0.0.1:{port}")).is_ok();
    if is_port_free {
        Command::new(server_path)
            .current_dir(app_data())
            .kill_on_drop(false)
            .spawn()?;

        println!("{}", "Running".green());
    } else {
        println!("{}", "Already running".cyan());
    }

    println!(
        "{}",
        "─".repeat(UNDERLINE_COUNT).color(Color::AnsiColor(240))
    );
    println!(" {}\n", "System is ready for requests.".italic().color(dim));

    Ok(())
}
