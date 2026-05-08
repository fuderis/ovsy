pub mod chat;
pub mod config;
pub mod restart;
pub mod start;
pub mod status;
pub mod stop;
pub mod update;

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
