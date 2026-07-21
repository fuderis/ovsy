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

pub mod session_id;
pub use session_id::SessionId;

pub mod session_info;
pub use session_info::SessionInfo;

pub mod skill;
pub use skill::Skill;

pub mod event;
pub use event::{Event, EventKind, EventTaskInfo};

pub mod user_query;
pub use user_query::{CompactQuery, HandleQuery, UserSessionsQuery};

pub mod agent_metadata;
pub use agent_metadata::AgentMetadata;

pub mod status_data;
pub use status_data::StatusData;

pub fn macos_protect() {
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
}
