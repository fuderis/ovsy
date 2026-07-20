pub mod chat;
pub mod health;
pub mod system;

const UNDERLINE_SIZE: usize = 40;

/// Prints underline
pub(super) fn underline() {
    use colored::*;

    println!(
        "{}",
        "─".repeat(UNDERLINE_SIZE).color(Color::AnsiColor(240))
    );
}
