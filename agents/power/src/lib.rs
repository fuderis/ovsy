#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]
pub mod error;
pub use error::Error;
pub mod prelude;
pub mod settings;
pub use settings::Settings;
pub mod utils;

pub mod power;
pub use power::PowerMode;

pub mod handlers;

/// Returns path to app data dir
pub fn app_data() -> prelude::PathBuf {
    prelude::path!("~/.config/ovsy/agents/power")
}
