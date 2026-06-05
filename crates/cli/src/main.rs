// Copyright (C) 2026 Bulat Sh. (fuderis) <synapdrake@ya.ru>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use clap::{Parser, Subcommand};
use colored::*;
use ovsy_cli::{commands, prelude::*};
use ovsy_share::app_version;

/// The Ovsy CLI commands parser
#[derive(Parser)]
#[command(name = "ovsy")]
#[command(version = app_version())]
#[command(about = "Ovsy Assistant - Ecosystem Controller & Client", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

/// The Ovsy CLI commands
#[derive(Subcommand)]
enum Commands {
    /// Check the status of all ecosystem components
    Status,

    /// Start the Ovsy server in the background
    Start {
        /// Also run the LM Studio server and load models
        #[arg(short, long)]
        lms: bool,
    },

    /// Stop the Ovsy server by killing the port process
    Stop {
        /// Also stop the LM Studio server and unload models
        #[arg(short, long)]
        lms: bool,
    },

    /// Restart the ecosystem (stop -> start)
    Restart {
        #[arg(short, long)]
        lms: bool,
    },

    /// Update the server settings & agents list
    Update,

    /// Enter interactive AI chat mode
    Chat,

    /// Open settings.toml in the default system editor
    #[command(alias = "conf")]
    Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    // parse arguments:
    let cli = Cli::parse();

    // initialize settings:
    Settings::init(app_data().join("config/settings.toml"))
        .await
        .ok();

    if let Err(e) = match cli.command {
        Some(Commands::Status) => commands::status::handle().await,
        Some(Commands::Start { lms }) => commands::start::handle(lms).await,
        Some(Commands::Stop { lms }) => commands::stop::handle(lms).await,
        Some(Commands::Restart { lms }) => commands::restart::handle(lms).await,
        Some(Commands::Update) => commands::update::handle().await,
        Some(Commands::Chat) | None => commands::chat::handle().await,
        Some(Commands::Config) => commands::config::handle().await,
    } {
        eprintln!("\n{}: {}", "Error".red().bold(), e.to_string().white());
        std::process::exit(1);
    }

    Ok(())
}
