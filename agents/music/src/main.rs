use agent::AgentServer;
use root::{handlers, prelude::*};

#[tokio::main]
async fn main() -> Result<()> {
    // init logger:
    Logger::init(app_data().join("logs"), 20)?;

    // run server:
    AgentServer::new()
        .route("/play", handlers::play::handle)
        .route("/volume", handlers::volume::handle)
        .run()
        .await
}
