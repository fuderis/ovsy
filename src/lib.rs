#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]
pub mod error;       pub use error::{ StdResult, Result, Error };
pub mod prelude;     use prelude::{ PathBuf, path };
pub mod settings;    pub use settings::Settings;
pub mod manifest;    pub use manifest::Manifest;

pub mod tools;       pub use tools::{ Tools, Tool };
pub mod handlers;
pub mod llm;

/// Returns path to app data dir
pub fn app_data() -> PathBuf {
    path!("%/ovsy")
}
