pub mod prelude;

pub mod handlers;

pub mod power;
pub use power::{PowerMode, PowerOptions};

use atoman::State;

/// The deferred power operation
pub static ACTIVE_ACTION: State<Option<(u128, PowerMode)>> = State::new();
