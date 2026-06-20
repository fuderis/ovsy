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

use ovsy_core::{Manager, handlers, prelude::*};
use pearce::Server;

#[tokio::main]
async fn main() -> Result<()> {
    // init settings & logger:
    Settings::init(app_data().join("config/settings.toml")).await?;
    Logger::init(app_data().join("logs"), Settings::get().server.max_logs).await?;

    // init agents manager:
    Manager::init().await?;

    // start server:
    Server::new()
        .post("/users/{uid}/sessions", handlers::user_sessions)
        .post("/sessions/{sid}/get", handlers::session_get)
        .post("/sessions/{sid}/compact", handlers::session_compact)
        .post("/sessions/{sid}/clear", handlers::session_clear)
        .post("/sessions/{sid}/query", handlers::session_query)
        .post("/status", handlers::status)
        .post("/update", handlers::update)
        .run(Settings::get().server.port)
        .await?;

    Ok(())
}
