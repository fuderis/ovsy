pub mod chunk;
pub use chunk::*;

pub mod user_query;
pub use user_query::*;

pub mod settings;
pub use settings::Settings;

// The actual Ovsy version
pub const VERSION: &str = "0.7.0";

/// Returns the app data dir
pub fn app_data() -> std::path::PathBuf {
    fuderis_basic::path!("~/.ovsy")
}

/// Returns the app version
pub fn app_version() -> &'static str {
    VERSION
}
