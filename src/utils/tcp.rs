use crate::prelude::*;

/// Returns a free local port
pub async fn get_free_port() -> Result<u16> {
    use tokio::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    Ok(listener.local_addr()?.port())
}
