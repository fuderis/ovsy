use ovsy_server::{handlers, prelude::*};

#[tokio::main]
async fn main() -> Result<()> {
    // init settings & logger:
    Settings::init(app_data().join("settings.toml")).await?;
    Logger::init(app_data().join("logs"), Settings::get().server.logs_limit).await?;

    // start server:
    Server::new()
        .post("/handle", handlers::query_handler)
        .post("/refresh", handlers::refresh_handler)
        .post("/compress", handlers::compress_handler)
        .run(([127, 0, 0, 1], Settings::get().server.port))
        .await?;

    Ok(())
}
