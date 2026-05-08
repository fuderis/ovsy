use clap::{Parser, Subcommand};
use colored::*;
use ovsy_cli::{commands, prelude::*};

/// The Ovsy CLI commands parser
#[derive(Parser)]
#[command(name = "ovsy")]
#[command(version = "0.7.0")]
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
    Settings::init(macron::path!("$/../config/settings.toml"))
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
