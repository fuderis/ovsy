use crate::{UNDERLINE_COUNT, prelude::*};
use colored::*;
use ovsy_shared::RefreshResponse;
use reqwest::Client;
use std::io::{self, Write};

pub async fn refresh() -> Result<()> {
    let cyan = Color::Cyan;
    let dim = Color::AnsiColor(247);
    let port = Settings::get().server.port;

    print!("📡 Connecting to Ovsy server... ");
    io::stdout().flush().ok();

    let client = Client::new();
    let res = client
        .post(format!("http://127.0.0.1:{port}/refresh"))
        .send()
        .await;

    match res {
        Ok(response) => {
            println!("{}", "Connected".green());

            let data: RefreshResponse = response
                .json()
                .await
                .map_err(|e| str!(format!("Failed to parse response: {e}")))?;

            match data {
                RefreshResponse::Error { error } => {
                    println!(" {} {}", "❌".red(), error.red());
                }
                RefreshResponse::Success { agents } => {
                    println!("⚙️  {}", "Settings synchronized.".bright_white());

                    if agents.is_empty() {
                        println!("\n⚠️  {}", "No agents loaded.".dimmed());
                    } else {
                        println!("\n {}", "Loaded Agents:".bold());
                        for (name, enabled) in agents {
                            let status = if enabled {
                                "Enabled".green()
                            } else {
                                "Disabled".color(Color::AnsiColor(240))
                            };
                            println!(
                                "  {} {:<20} {}",
                                "•".color(cyan),
                                name.bright_white(),
                                status
                            );
                        }
                    }
                }
            }
        }
        Err(_) => {
            println!("{}", "Offline".red());
            return Err(str!("Server is not responding. Check if it's running.").into());
        }
    }

    println!(
        "{}",
        "─".repeat(UNDERLINE_COUNT).color(Color::AnsiColor(240))
    );
    println!("{}\n", "Environment synchronized.".italic().color(dim));

    Ok(())
}
