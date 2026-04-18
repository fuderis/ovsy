use clap::{Parser, Subcommand};
use colored::*;
use ovsy_cli::{cmds, prelude::*};

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
    Build,

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
        Some(Commands::Build) => cmds::build().await,
        Some(Commands::Status) => cmds::status().await,
        Some(Commands::Start) => cmds::start().await,
        Some(Commands::Stop { full }) => cmds::stop(full).await,
        Some(Commands::Restart { full }) => cmds::restart(full).await,
        Some(Commands::Chat) | None => cmds::chat().await,
        Some(Commands::Config) => cmds::config().await,
    } {
        eprintln!(
            "\n {} {}: {}",
            "×".red().bold(),
            "Error".red().bold(),
            e.to_string().white()
        );
        std::process::exit(1);
    }

    Ok(())
}
