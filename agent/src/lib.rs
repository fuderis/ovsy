pub use macron;

pub mod error;
pub use error::Error;
pub mod prelude;

pub mod chunk;
pub use chunk::SessionChunk;

pub mod session;
pub use session::Session;

use crate::prelude::*;
use axum::{
    Json, Router,
    routing::{get, post},
};
use clap::Parser;
use serde_json::{Value, json};
use std::net::SocketAddr;
use tokio::net::TcpListener;

/// Standard arguments for all agents
#[derive(Parser, Debug)]
pub struct AgentArgs {
    #[arg(short, long)]
    pub port: u16,
}

pub struct AgentServer {
    router: Router,
}

impl AgentServer {
    pub fn new() -> Self {
        // basic router with default endpoints:
        let router = Router::new()
            .route("/", get(|| async { Html("") }))
            .route("/health", post(Self::health_handler));

        Self { router }
    }

    /// Adds a POST route (the standard for invoking agent actions)
    pub fn route<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: axum::handler::Handler<T, ()>,
        T: 'static,
    {
        self.router = self.router.route(path, post(handler));
        self
    }

    /// The standard health handler
    async fn health_handler() -> Json<Value> {
        Json(json!({
            "log_file": Logger::get_path()
        }))
    }

    /// Launching a server with automatic port parsing
    pub async fn run(self) -> Result<()> {
        let args = AgentArgs::parse();
        let addr = SocketAddr::from(([127, 0, 0, 1], args.port));

        let listener = TcpListener::bind(addr).await.map_err(|e| {
            error!("Agent could not bind to port {}: {}", args.port, e);
            e
        })?;

        info!("🚀 Agent is running on http://{}", addr);
        axum::serve(listener, self.router).await?;

        Ok(())
    }
}
