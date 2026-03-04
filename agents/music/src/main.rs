use root::{handlers, prelude::*};

use axum::{
    Router,
    routing::{get, post},
};
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<()> {
    Logger::init(app_data().join("logs"), 20)?;
    Settings::init(app_data().join("config.toml"))?;

    // create router:
    let router = Router::new()
        .route("/", get(async || Html("")))
        .route("/play", post(handlers::play::handle))
        .route("/volume", post(handlers::volume::handle));

    // init listenner:
    let port = Settings::get().server.port;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    info!("🚀 Serve music agent on http://{addr}..");

    let listener = TcpListener::bind(addr).await.map_err(|e| {
        error!("Error with running server: {e}");
        e
    })?;

    // run server:
    axum::serve(listener, router).await?;

    Ok(())
}
