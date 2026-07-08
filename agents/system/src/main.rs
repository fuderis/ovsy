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

use pearce::Server;
use system_agent::{handlers, prelude::*};

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        tokio::spawn(async {
            use tokio::io::AsyncReadExt;
            let mut std_in = tokio::io::stdin();
            let mut buf = [0; 1];
            if let Ok(0) = std_in.read(&mut buf).await {
                std::process::exit(0);
            }
        });
    }

    // init settings && logger:
    Settings::init(path!("~/.config/ovsy/system.toml")).await?;
    Logger::init(
        path!("~/.cache/ovsy/logs/system"),
        Settings::get().server.max_logs,
    )
    .await?;

    // start server:
    let sock = path!("/tmp/ovsy/uds/{}.sock", Settings::get().agent.name);
    Server::new()
        .post("/ping", handlers::handle_ping)
        .post("/info", handlers::handle_info)
        .post("/tools/call/{tool}", handlers::handle_tool_call)
        .run(sock)
        .await
}
