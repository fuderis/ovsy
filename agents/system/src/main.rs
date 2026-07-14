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
pub mod settings;

pub mod handlers;
pub mod skills;
pub mod tools;

use pearce::Server;
use prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    use handlers as hands;

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
    Settings::init(path!("$config$/system.toml")).await?;
    Logger::init(
        path!("$cache$/logs/system"),
        Settings::get().server.max_logs,
    )
    .await?;

    // start server:
    let sock = path!("$temp$/uds/{}.sock", Settings::get().metadata.name);
    Server::new()
        .post("/ping", hands::init::handle_ping)
        .post("/init", hands::init::handle_init)
        .post("/tools/list", hands::tools::handle_tools_list)
        .post("/tools/call/{tool}", hands::tools::handle_tool_call)
        .run(sock)
        .await
}
