#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]
pub mod error;
pub use error::{Error, Result, StdResult};
pub mod prelude;
use prelude::{PathBuf, path};
pub mod settings;
pub use settings::{LMKind, Settings};
pub mod manifest;
pub use manifest::Manifest;

pub mod session;
pub use session::SessionLog;
pub mod tools;
pub use tools::{Tool, ToolCall, Tools};
pub mod handlers;
pub mod lms;

/// Returns path to app data dir
pub fn app_data() -> PathBuf {
    path!("~/.config/ovsy")
}
