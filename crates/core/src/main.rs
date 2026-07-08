// Copyright (C) 2026 Bulat Sh. (fuderis) <synapdrake@ya.ru>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

pub mod error;
pub mod prelude;

pub mod runtime;
pub use runtime::Runtime;

pub mod manager;
pub use manager::{Agent, Manager};

pub mod session;
pub use session::Session;

pub mod handlers;

/// Returns a free local port
pub async fn free_port() -> prelude::Result<u16> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    Ok(listener.local_addr()?.port())
}

use pearce::Server;
use prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    use handlers as hands;

    // init settings & logger:
    Settings::init(path!("~/.config/ovsy/settings.toml")).await?;
    Logger::init(path!("~/.cache/ovsy/logs"), Settings::get().server.max_logs).await?;

    // init agents manager:
    Manager::init().await?;

    // start server:
    Server::new()
        //    USERS
        .post("/users/{uid}/sessions", hands::user::handle_list)
        //    SESSIONS
        .post("/sessions/{sid}/init", hands::session::handle_init)
        .post("/sessions/{sid}/finish", hands::session::handle_finish)
        .post("/sessions/{sid}/compact", hands::session::handle_compact)
        .post("/sessions/{sid}/clear", hands::session::handle_clear)
        //    QUERY
        .post("/sessions/{sid}/query", hands::query::handle_query)
        //    HEALTH
        .post("/status", hands::health::handle_status)
        .post("/refresh", hands::health::handle_refresh)
        .run(Settings::get().server.port)
        .await?;

    Ok(())
}
