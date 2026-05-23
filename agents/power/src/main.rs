use power_agent::{handlers, prelude::*};

use clap::Parser;
use pearce::Server;

/// Agent arguments
#[derive(Parser, Debug)]
pub struct Args {
    /// The server running port
    #[arg(short, long)]
    pub port: u16,

    /// The max count of log-files
    #[arg(short, long)]
    pub max_logs: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // init logger & settings:
    Logger::init(app_data().join("logs/power"), args.max_logs).await?;

    // start server:
    Server::new()
        .post("/health", handlers::health::handle)
        .post("/tool/power", handlers::power::handle)
        .post("/tool/cancel", handlers::cancel::handle)
        .run(args.port)
        .await
}
