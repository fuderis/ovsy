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

use system_agent::{handlers, prelude::*};

use clap::Parser;
use pearce::Server;

/// Agent arguments
#[derive(Parser, Debug)]
pub struct Args {
    #[arg(short, long)]
    pub port: u16,
    #[arg(short, long)]
    pub max_logs: usize,
}

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

    let args = Args::parse();

    // init logger & settings:
    Logger::init(app_data().join("logs/system"), args.max_logs).await?;
    Settings::init(app_data().join("config/system.toml")).await?;

    // start server:
    Server::new()
        .post("/ping", handlers::handle_ping)
        .post("/info", handlers::handle_info)
        .post("/tools/call/{tool}", handlers::handle_tool_call)
        .run(args.port)
        .await
}
