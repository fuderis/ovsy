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

pub const APP_NAME: &str = "ovsy";
pub const APP_VERSION: &str = "0.13.0";

pub mod result;
pub use result::{DynError, Result, StdResult};

pub mod settings;
pub use settings::Settings;

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

pub mod agent_task;
pub use agent_task::AgentTask;

pub mod status_data;
pub use status_data::StatusData;

pub mod ping_data;
pub use ping_data::PingData;
