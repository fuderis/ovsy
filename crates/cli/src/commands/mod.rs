pub mod chat;
pub mod health;
pub mod system;

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
