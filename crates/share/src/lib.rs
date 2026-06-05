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

/// Returns the app data dir
pub fn app_data() -> std::path::PathBuf {
    macron::path!("~/.ovsy")
}
/// Returns the app version
pub fn app_version() -> &'static str {
    "0.7.5"
}

pub mod result;
pub use result::{DynError, Result, StdResult};

pub mod chunk;
pub use chunk::{Chunk, ChunkData};

pub mod user_query;
pub use user_query::*;

pub mod settings;
pub use settings::Settings;

pub mod agent_info;
pub use agent_info::AgentInfo;

pub mod agent_task;
pub use agent_task::AgentTask;

pub mod status_data;
pub use status_data::StatusData;

pub mod ping_data;
pub use ping_data::PingData;
