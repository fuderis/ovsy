use root::{handlers, prelude::*};
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<()> {
    Logger::init(app_data().join("logs"), 20)?;
    Settings::init(app_data().join("config.toml"))?;

    // create router:
    let router = Router::new()
        .route("/", get(async || Html("")))
        .route("/wait", post(handlers::wait::handle))
        .route("/power", post(handlers::power::handle))
        .route("/app", post(handlers::app::handle))
        .route("/music", post(handlers::music::handle))
        .route("/volume", post(handlers::volume::handle));

    // init listenner:
    let port = Settings::get().server.port;
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    info!("🚀 Serve 'system' tool on http://{address}..");

    let listener = TcpListener::bind(address).await.map_err(|e| {
        error!("Error with running server: {e}");
        e
    })?;

    // run server:
    axum::serve(listener, router).await?;

    Ok(())
}
