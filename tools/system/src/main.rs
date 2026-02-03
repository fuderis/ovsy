use root::{handlers, prelude::*};
use std::net::SocketAddr;
use tokio::{
    net::TcpListener,
    time::{Duration, sleep},
};

#[tokio::main]
async fn main() -> Result<()> {
    Logger::init(app_data().join("logs"), 20)?;
    Settings::init(app_data().join("config.toml"))?;

    // create router:
    let router = Router::new()
        .route("/", get(async || Html("")))
        .route("/power", post(handlers::power::handle))
        .route("/app", post(handlers::app::handle))
        .route("/play", post(handlers::play::handle))
        .route("/volume", post(handlers::volume::handle));

    // init listenner:
    let port = Settings::get().server.port;
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    info!("🚀 Serve 'system' tool on http://{address}..");

    let listener = loop {
        match TcpListener::bind(address).await {
            Ok(r) => break r,
            Err(e) => {
                warn!("Error with running server: {e}");
                sleep(Duration::from_millis(600)).await;
            }
        }
    };

    // run server:
    axum::serve(listener, router).await?;

    Ok(())
}
