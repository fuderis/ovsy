#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]
pub mod error;       pub use error::{ StdResult, Result, Error };
pub mod prelude;     use prelude::{ PathBuf, path };
pub mod settings;    pub use settings::Settings;

pub mod handlers;

/// Returns path to app data dir
pub fn app_data() -> PathBuf {
    path!("%/ovsy/pc-control")
}
