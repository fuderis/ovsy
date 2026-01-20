use root::{handlers, prelude::*};

#[tokio::main]
async fn main() -> Result<()> {
    Logger::init(app_data().join("logs"), 20)?;
    Settings::init(app_data().join("config/settings.toml"))?;

    // check arguments:
    let port = Settings::get().server.port;
    let query = std::env::args().collect::<Vec<_>>()[1..]
        .join(" ")
        .trim()
        .to_owned();
    if !query.is_empty() {
        let client = reqwest::Client::new();
        let result = client
            .post(fmt!("http://localhost:{port}/query"))
            .json(&json!({ "query": query }))
            .send()
            .await?;

        if result.status() != 200 {
            err!("{}", result.text().await?);
        }
        return Ok(());
    }

    // manage tools:
    Tools::manage(Settings::get().tools.timeout);

    // create router:
    let router = Router::new()
        .route("/query", post(handlers::query::handle))
        .route("/{name}/{action}", post(handlers::tool::handle));

    // init listenner:
    let address = SocketAddr::from(([127, 0, 0, 1], Settings::get().server.port));
    info!("🚀 Running on 'http://{address}'..");

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
