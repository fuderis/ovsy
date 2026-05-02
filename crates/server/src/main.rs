use ovsy_server::{Manager, handlers, prelude::*};
use pearce::Server;

#[tokio::main]
async fn main() -> Result<()> {
    // init settings & logger:
    Settings::init(app_data().join("settings.toml")).await?;
    Logger::init(app_data().join("logs"), Settings::get().server.max_logs).await?;

    // init agents manager:
    Manager::init().await?;

    // start server:
    Server::new()
        .post("/handle", handlers::query::handle)
        .post("/compact", handlers::compact::handle)
        .post("/status", handlers::status::handle)
        .post("/refresh", handlers::refresh::handle)
        .run(Settings::get().server.port)
        .await?;

    Ok(())
}
