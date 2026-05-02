pub use std::result::Result as StdResult;

/// The dynamic error type
pub type DynError = Box<dyn std::error::Error + Send + Sync + 'static>;
/// The short result alias
pub type Result<T> = StdResult<T, DynError>;
