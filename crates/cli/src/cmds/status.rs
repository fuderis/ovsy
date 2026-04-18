use crate::prelude::*;
use colored::*;
use tokio::process::Command;

/// Handles the `status` command
pub async fn status() -> Result<()> {
    let cyan = Color::Cyan;
    let port = Settings::get().server.port;

    // 1. Ovsy Server
    let is_port_free = std::net::TcpListener::bind(format!("127.0.0.1:{port}")).is_ok();
    let server_status = if is_port_free {
        "Offline".red()
    } else {
        format!("Online (port {port})").green()
    };
    println!(
        " {} {}   {}",
        "⚡".color(cyan),
        "Ovsy Server:".bold(),
        server_status
    );

    // 2. LM Studio Status & Port
    let lms_out = Command::new("lms").args(["status"]).output().await;
    let lms_raw = match lms_out {
        Ok(out) => String::from_utf8_lossy(&out.stdout).to_string(),
        _ => String::new(),
    };

    let lms_running = lms_raw.contains("ON");

    // parsing port:
    let lms_port = lms_raw
        .lines()
        .find(|l| l.contains("port:"))
        .and_then(|l| l.split("port:").last())
        .map(|p| p.trim_matches(|c: char| !c.is_numeric()))
        .unwrap_or("unknown");

    let lms_display = if lms_running {
        format!("Online (port {lms_port})").green()
    } else {
        "Offline".red()
    };
    println!(
        " {} {}     {}",
        "📡".color(cyan),
        "LM Studio:".bold(),
        lms_display
    );

    // loaded models:
    if lms_running {
        let mut in_models_block = false;
        let mut found_any = false;

        for line in lms_raw.lines() {
            let line = line.trim();

            if line.contains("Loaded Models") {
                in_models_block = true;
                println!(" {} Loaded Models:", "🧠".color(cyan));
                continue;
            }

            if in_models_block && line.starts_with('·') {
                found_any = true;
                let model_info = line.trim_start_matches('·').trim();
                println!("    {}", model_info.dimmed());
            }
        }

        if !found_any && in_models_block {
            println!("    {}", "None".yellow().dimmed());
        }
    }

    Ok(())
}
