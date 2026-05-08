pub mod result;
pub use result::*;

pub mod chunk;
pub use chunk::*;

pub mod user_query;
pub use user_query::*;

pub mod settings;
pub use settings::Settings;

pub mod manifest;
pub use manifest::Manifest;

pub mod agent_info;
pub use agent_info::AgentInfo;

pub mod response;
pub use response::*;

/// Returns the app data dir
pub fn app_data() -> std::path::PathBuf {
    macron::path!("~/.ovsy")
}

/// Returns the app version
pub fn app_version() -> &'static str {
    "0.7.0"
}
