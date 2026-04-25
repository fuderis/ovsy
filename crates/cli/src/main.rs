use clap::{Parser, Subcommand};
use colored::*;
use ovsy_cli::{commands, prelude::*};

/// The Ovsy CLI commands parser
#[derive(Parser)]
#[command(name = "ovsy")]
#[command(version = "0.7.0")]
#[command(about = "Ovsy AI Ecosystem Controller & Client", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

/// The Ovsy CLI commands
#[derive(Subcommand)]
enum Commands {
    /// Build and install Ovsy to app_data
    Build {
        #[arg(short, long)]
        start: bool,
    },

    /// Check the status of all ecosystem components
    Status,

    /// Start the Ovsy server in the background
    Start,

    /// Stop the Ovsy server by killing the port process
    Stop {
        /// Also stop the LM Studio server and unload models
        #[arg(short, long)]
        full: bool,
    },

    /// Restart the ecosystem (stop -> start)
    Restart {
        #[arg(short, long)]
        full: bool,
    },

    /// Refresh the server settings & agents list
    Refresh,

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
    Settings::init(app_data().join("settings.toml")).await.ok();

    if let Err(e) = match cli.command {
        Some(Commands::Build { start }) => commands::build::handle(start).await,
        Some(Commands::Status) => commands::status::handle().await,
        Some(Commands::Start) => commands::start::handle().await,
        Some(Commands::Stop { full }) => commands::stop::handle(full).await,
        Some(Commands::Restart { full }) => commands::restart::handle(full).await,
        Some(Commands::Refresh) => commands::refresh::handle().await,
        Some(Commands::Chat) | None => commands::chat::handle().await,
        Some(Commands::Config) => commands::config::handle().await,
    } {
        eprintln!("\n{}: {}", "Error".red().bold(), e.to_string().white());
        std::process::exit(1);
    }

    Ok(())
}
