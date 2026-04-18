use crate::{UNDERLINE_COUNT, prelude::*};
use colored::*;
use std::{
    env, fs,
    io::{self, Write},
};
use tokio::process::Command;

/// Handles the `build` command
pub async fn build() -> Result<()> {
    let rust_orange = Color::AnsiColor(209);
    let cyan = Color::Cyan;

    println!("{} {}", "🛠️".color(cyan), "Building Ovsy Ecosystem".bold());

    // search root directory:
    if cfg!(debug_assertions) && !Path::new(".git").exists() {
        println!("Root not found, searching parent directories...");
        for _ in 0..3 {
            if env::set_current_dir("..").is_err() {
                break;
            }
            if Path::new(".git").exists() {
                println!("Project root detected at: {:?}", env::current_dir().ok());
                break;
            }
        }
    }

    if !Path::new(".git").exists() {
        return Err(str!(".git directory not found.").into());
    }

    // 2. KILL EXISTING PROCESSES to prevent file locking:
    Settings::init(app_data().join("settings.toml")).await.ok();
    let port = Settings::get().server.port;
    print!(" {} Cleaning port {}... ", "🧹".color(cyan), port);
    io::stdout().flush().ok();

    #[cfg(unix)]
    {
        let _ = Command::new("sh")
            .arg("-c")
            .arg(format!("fuser -k {}/tcp", port))
            .output()
            .await;
    }
    #[cfg(windows)]
    {
        let cmd = format!(
            "for /f \"tokens=5\" %a in ('netstat -aon ^| findstr \":{}\"') do taskkill /f /pid %a",
            port
        );
        let _ = Command::new("cmd").args(["/C", &cmd]).output().await;
    }
    println!("{}", "Clean".green());

    println!(
        " {} Running Cargo Build (Release)...",
        "📦".color(rust_orange)
    );
    println!(
        "{}",
        "─".repeat(UNDERLINE_COUNT).color(Color::AnsiColor(240))
    );

    let status = Command::new("cargo")
        .args(["build", "--release"])
        .status()
        .await
        .map_err(|e| str!("Cargo execution error: {e}"))?;

    if !status.success() {
        return Err(str!("Cargo build failed.").into());
    }

    println!(
        "{}",
        "─".repeat(UNDERLINE_COUNT).color(Color::AnsiColor(240))
    );
    println!(" {} Deploying binaries...", "🚚".color(cyan));

    let install_dir = app_data();
    fs::create_dir_all(&install_dir).ok();

    let exe = env::consts::EXE_SUFFIX;
    let binaries = [
        (
            format!("target/release/ovsy-cli{exe}"),
            format!("ovsy-cli{exe}"),
        ),
        (
            format!("target/release/ovsy-server{exe}"),
            format!("ovsy-server{exe}"),
        ),
    ];

    for (src_str, name) in binaries {
        let src = Path::new(&src_str);
        let dest = install_dir.join(&name);

        if src.exists() {
            if dest.exists() {
                let backup_path = dest.with_extension("old");

                if backup_path.exists() {
                    let _ = fs::remove_file(&backup_path);
                }

                // rename old file:
                if let Err(e) = fs::rename(&dest, &backup_path) {
                    println!(
                        "    {} {} -> {} ({})",
                        "⚠".yellow(),
                        name.dimmed(),
                        "backup failed".yellow(),
                        e
                    );
                }
            }

            // dest is free, copy new file:
            if let Err(e) = fs::copy(src, &dest) {
                println!(
                    "    {} {} -> {} ({})",
                    "✘".red(),
                    name.bright_white(),
                    "failed to copy".red(),
                    e
                );
                continue;
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&dest, fs::Permissions::from_mode(0o755));
            }

            println!(
                "    {} {} -> {}",
                "✔".green(),
                name.bright_white(),
                "installed".dimmed()
            );

            // remove old file:
            let backup_path = dest.with_extension("old");
            if backup_path.exists() {
                let _ = fs::remove_file(backup_path).ok();
            }
        }
    }

    println!("\n {} {}", "✨".yellow(), "Build successful!".bold());
    Ok(())
}
