use system_agent::{handlers, prelude::*};

use clap::Parser;
use pearce::Server;

/// Agent arguments
#[derive(Parser, Debug)]
pub struct Args {
    #[arg(short, long)]
    pub port: u16,
    #[arg(short, long)]
    pub max_logs: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // init logger & settings:
    Logger::init(app_data().join("logs/system"), args.max_logs).await?;
    Settings::init(app_data().join("config/system.toml")).await?;

    // start server:
    Server::new()
        .post("/ping", handlers::ping::handle)
        .post("/info", handlers::info::handle)
        .post("/call/{tool}", handlers::call::handle)
        .run(args.port)
        .await
}
