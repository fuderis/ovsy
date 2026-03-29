#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]
pub mod error;
pub use error::{Error, Result, StdResult};
pub mod prelude;
use prelude::{PathBuf, path};
pub mod settings;
pub use settings::Settings;
pub mod database;
pub use database::{Database, Record};
pub mod utils;

pub mod manifest;
pub use manifest::Manifest;
pub mod agents;
pub use agents::{Agent, Agents};

pub mod session;
pub use session::{CachedQuery, Session, SessionChunk};

pub mod handlers;

/// Returns path to app data dir
pub fn app_data() -> PathBuf {
    path!("~/.config/ovsy")
}
