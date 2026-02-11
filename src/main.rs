use root::{handlers, prelude::*};
use serde_json::json;
use std::net::SocketAddr;
use tokio::{
    net::TcpListener,
    time::{Duration, sleep},
};

#[tokio::main]
async fn main() -> Result<()> {
    Logger::init(app_data().join("logs"), 20)?;
    Settings::init(app_data().join("settings.toml"))?;

    let port = Settings::get().server.port;

    // check arguments:
    let query = std::env::args().collect::<Vec<_>>()[1..].join(" ");
    if !query.is_empty() {
        // send response:
        let response = reqwest::Client::new()
            .post(format!("http://localhost:{port}/query"))
            .json(&json!({ "query": query, "session_id": "root" }))
            .send()
            .await?;

        // stream response:
        use futures::StreamExt;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk?;
            let text = String::from_utf8_lossy(&bytes);

            eprint!("{text}");
        }
        println!();
        return Ok(());
    }

    // manage tools:
    Tools::manage();

    // create router:
    let router = Router::new()
        .route("/query", post(handlers::query::handle))
        .route("/call/{name}/{action}", post(handlers::call::handle));

    // init listener:
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    info!("🚀 Running Ovsy on http://{address}..");

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
