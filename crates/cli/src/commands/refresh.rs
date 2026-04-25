use crate::{UNDERLINE_COUNT, prelude::*};
use colored::*;
use ovsy_shared::{AgentInfo, StatusResponse};
use reqwest::Client;
use std::io::{self, Write};

pub async fn handle() -> Result<()> {
    let dim = Color::AnsiColor(247);
    let port = Settings::get().server.port;

    // Начало вывода
    print!("📡 {} ", "Refreshing Ovsy server...".bold());
    io::stdout().flush().ok();

    let client = Client::new();
    let res = client
        .post(format!("http://127.0.0.1:{port}/refresh"))
        .send()
        .await;

    match res {
        Ok(response) => {
            // Если сервер ответил, значит он Online
            println!("{}", format!("Online (port {port})").green());

            let data: StatusResponse = response
                .json()
                .await
                .map_err(|e| str!(format!("Failed to parse response: {e}")))?;

            match data {
                StatusResponse::Success { agents } => {
                    // Вывод агентов в стиле списка LM Studio
                    if agents.is_empty() {
                        println!("   {}", "No agents loaded".yellow().dimmed());
                    } else {
                        for AgentInfo { name, .. } in agents {
                            println!(" • {}", name.dimmed());
                        }
                    }

                    println!("\n⚙️  {}", "Settings synchronized.".bright_white());
                }
                StatusResponse::Error { error } => {
                    println!("   {} {}", "❌ Error:".red(), error.red());
                }
            }
        }
        Err(_) => {
            // Если сервер не отвечает
            println!("{}", "Offline".red());
            return Err(str!("Server is not responding. Check if it's running.").into());
        }
    }

    // Футер
    println!(
        "{}",
        "─".repeat(UNDERLINE_COUNT).color(Color::AnsiColor(240))
    );
    println!("{}\n", "Environment synchronized.".italic().color(dim));

    Ok(())
}
