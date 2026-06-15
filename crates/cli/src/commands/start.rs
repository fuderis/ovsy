use crate::prelude::*;
use anylm::ApiKind;
use colored::*;
use std::{
    io::{self, Write},
    net::TcpListener,
    process::Stdio,
};
use tokio::process::Command;

/// API: Handles the `start` command
pub async fn handle_start(start_lms: bool) -> Result<()> {
    let server_path = path!("$/ovsy-core{}", if cfg!(windows) { ".exe" } else { "" });

    if !server_path.exists() {
        return Err(str!("Server binary missing. Please, re-install Ovsy.").into());
    }

    // running Ovsy server:
    let port = Settings::get().server.port;
    print!("Starting Ovsy server... ");
    io::stdout().flush().ok();

    // check port for busy:
    let is_port_free = TcpListener::bind(str!("127.0.0.1:{port}")).is_ok();
    if is_port_free {
        Command::new(server_path)
            .current_dir(app_data())
            .kill_on_drop(false)
            .spawn()?;
    }
    println!("{}", "Online".green());

    // running LMS server:
    let ai_conf = &Settings::get().assistant;
    if start_lms
        && (ai_conf.completions.kind == ApiKind::LmStudio
            || ai_conf.embeddings.kind == ApiKind::LmStudio)
    {
        print!("Starting LMS server... ");
        io::stdout().flush().ok();

        let is_running = match Command::new("lms").args(["status"]).output().await {
            Ok(out) => String::from_utf8_lossy(&out.stdout).contains("ON"),
            _ => false,
        };

        if !is_running {
            let bin_path = if cfg!(windows) { "lms.exe" } else { "lms" };

            match Command::new(bin_path)
                .args(["server", "start"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(_child) => {
                    let mut is_ok = false;

                    // 100 tries * 100 мс = 10 seconds to start:
                    for _ in 0..100 {
                        time::sleep(Duration::from_millis(100)).await;

                        let status_check = Command::new(bin_path).args(["status"]).output().await;

                        if let Ok(out) = status_check {
                            if String::from_utf8_lossy(&out.stdout).contains("ON") {
                                is_ok = true;
                                break;
                            }
                        }
                    }

                    if is_ok {
                        println!("{}", "Online".green());
                    } else {
                        println!("{}", "Failed".red());
                    }
                }
                Err(e) => {
                    error!("[lms_spawn] Сбой запуска процесса: {e}");
                    println!("{}", "Failed".red());
                }
            }
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
            print!(" ∟ Loading model {}... ", model.dimmed());
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

    super::underline();
    println!("{}\n", "Ready for requests!".italic().dimmed());

    Ok(())
}
