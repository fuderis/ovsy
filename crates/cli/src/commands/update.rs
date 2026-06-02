use crate::{UNDERLINE_COUNT, prelude::*};
use colored::*;
use ovsy_share::{AgentInfo, StatusData};
use reqwest::Client;
use std::io::{self, Write};

/// API: Handles the `udpate` command
pub async fn handle() -> Result<()> {
    let dim = Color::AnsiColor(247);
    let port = Settings::get().server.port;

    print!("{} ", "Updating Ovsy server...".bold());
    io::stdout().flush().ok();

    let client = Client::new();
    let res = client
        .post(str!("http://127.0.0.1:{port}/update"))
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
                        for AgentInfo { name, .. } in agents {
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

    println!(
        "{}",
        "─".repeat(UNDERLINE_COUNT).color(Color::AnsiColor(240))
    );
    println!("{}\n", "Environment synchronized.".italic().color(dim));

    Ok(())
}
