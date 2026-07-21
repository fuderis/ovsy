pub mod chat;
pub mod health;
pub mod system;

use colored::*;

pub fn section(title: &str) {
    println!();
    println!("{} {}", "▶".cyan().bold(), title.bold());
}

pub fn info(label: &str, message: &str) {
    if !label.is_empty() {
        println!(
            "  {} {}{} {}",
            "•".cyan(),
            label.bold(),
            ":".bold(),
            message
        );
    } else {
        println!("  {} {}", "•".cyan(), message);
    }
}

pub fn warn(message: &str) {
    if let Some((prefix, tail)) = message.split_once(": ") {
        println!(
            "  {} {}{} {tail}",
            "ℹ".yellow(),
            prefix.yellow(),
            ":".yellow()
        );
    } else {
        println!("  {} {}", "ℹ".yellow(), message);
    }
}

pub fn success(message: &str) {
    println!("  {} {}", "✓".green(), message);
}

pub fn error(e: crate::DynError) {
    if let Some((prefix, tail)) = crate::str!(e).split_once(": ") {
        println!("\n  {} {}{} {tail}", "✗".red(), prefix.red(), ":".red());
    } else {
        println!("\n  {} {e}", "✗".red());
    }
}
