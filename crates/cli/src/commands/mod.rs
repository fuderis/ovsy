pub mod chat;
pub use chat::handle_chat;

pub mod config;
pub use config::handle_config;

pub mod restart;
pub use restart::handle_restart;

pub mod start;
pub use start::handle_start;

pub mod status;
pub use status::handle_status;

pub mod stop;
pub use stop::handle_stop;

pub mod update;
pub use update::handle_update;

/// Prints underline
pub(super) fn underline() {
    use colored::*;

    println!(
        "{}",
        "─"
            .repeat(crate::UNDERLINE_COUNT)
            .color(Color::AnsiColor(240))
    );
}
