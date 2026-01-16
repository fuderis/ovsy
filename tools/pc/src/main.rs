use root::{ prelude::*, handlers };
use clap::Parser;

/// The launch arguments
#[derive(Parser)]
struct LaunchArgs {
    #[arg(short, long)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = LaunchArgs::parse();
    Logger::init(app_data().join("logs"), 20)?;
    Settings::init(app_data().join("config/settings.toml"))?;
    
    // create router:
    let router = Router::new()
        // .route("/", get(async || Html("Hello, World!")))
        .route("/play", post(handlers::play::handle))
        .route("/power", post(handlers::power::handle))
    ;

    // init listenner:
    let address = SocketAddr::from(([127,0,0,1], args.port));
    info!("🚀 Serve tool 'pc' on 'http://{address}'..");

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
