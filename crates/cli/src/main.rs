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

pub mod prelude;

pub mod chat;

pub mod commands;

pub const UNDERLINE_COUNT: usize = 40;

use clap::{Parser, Subcommand};
use colored::*;
use prelude::*;

/// The Ovsy CLI commands parser
#[derive(Parser)]
#[command(name = "ovsy")]
#[command(version = APP_VERSION)]
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
    /// Refreshes the server settings & agents list
    Refresh,

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

    /// Enter interactive AI chat mode
    Chat,

    /// Open settings.toml in the default system editor
    #[command(alias = "conf")]
    Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    use commands as cmds;

    // parse arguments:
    let cli = Cli::parse();

    // initialize settings:
    Settings::init(path!("$config$/settings.toml")).await.ok();

    if let Err(e) = match cli.command {
        //     SYSTEM
        Some(Commands::Start { lms }) => cmds::system::handle_start(lms).await,
        Some(Commands::Stop { lms }) => cmds::system::handle_stop(lms).await,
        Some(Commands::Restart { lms }) => cmds::system::handle_restart(lms).await,
        //     HEALTH
        Some(Commands::Status) => cmds::health::handle_status().await,
        Some(Commands::Refresh) => cmds::health::handle_refresh().await,
        Some(Commands::Config) => cmds::health::handle_config().await,
        //     CHAT
        Some(Commands::Chat) | None => cmds::chat::handle_chat().await,
    } {
        eprintln!("\n{}: {}", "Error".red().bold(), e.to_string().white());
        std::process::exit(1);
    }

    Ok(())
}
