pub mod prelude;

pub mod handlers;

/// Returns a free local port
pub async fn free_port() -> prelude::Result<u16> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    Ok(listener.local_addr()?.port())
}
