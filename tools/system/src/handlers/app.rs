use crate::prelude::*;
use tokio::process::Command;

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionMode {
    Open,
    Close,
}

/// The request POST data
#[derive(Deserialize)]
pub struct QueryData {
    mode: ActionMode,
    app: String,
}

/// Api '/app' handler
#[cfg(unix)]
pub async fn handle(Json(data): Json<QueryData>) -> impl IntoResponse {
    use libc::{ESRCH, SIGTERM, kill};

    match data.mode {
        ActionMode::Open => match Command::new(&data.app).spawn() {
            Ok(_) => (StatusCode::OK, "Application started".to_owned()),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                fmt!("Failed to start application: {e}"),
            ),
        },
        ActionMode::Close => {
            // Извлекаем базовое имя исполняемого файла
            let basename = Path::new(&data.app)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&data.app);

            // Ищем PID процессов с помощью pgrep
            let output = match Command::new("pgrep").arg("-x").arg(basename).output().await {
                Ok(out) => out,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        fmt!("Failed to execute pgrep: {e}"),
                    );
                }
            };

            if !output.status.success() {
                return (
                    StatusCode::NOT_FOUND,
                    "No matching process found".to_owned(),
                );
            }

            let pids = String::from_utf8_lossy(&output.stdout);
            let mut pids_list = Vec::new();

            for line in pids.lines() {
                if let Ok(pid) = line.trim().parse::<i32>() {
                    pids_list.push(pid);
                }
            }

            if pids_list.is_empty() {
                return (StatusCode::NOT_FOUND, "No valid PIDs found".to_owned());
            }

            let mut killed_count = 0;
            let mut errors = Vec::new();

            for pid in pids_list {
                let result = unsafe { kill(pid, SIGTERM) };
                if result == 0 {
                    killed_count += 1;
                } else {
                    let err = std::io::Error::last_os_error();
                    // ignoring ESRCH (process already finished)
                    if let Some(errno) = err.raw_os_error() {
                        if errno != ESRCH {
                            errors.push((pid, err.to_string()));
                        } else {
                            killed_count += 1;
                        }
                    } else {
                        errors.push((pid, "Unknown error".to_string()));
                    }
                }
            }

            if killed_count > 0 {
                let msg = if errors.is_empty() {
                    fmt!("Successfully terminated {} process(es)", killed_count)
                } else {
                    fmt!(
                        "Terminated {} process(es), {} failed: {:?}",
                        killed_count,
                        errors.len(),
                        errors
                            .iter()
                            .map(|(pid, err)| format!("PID {}: {}", pid, err))
                            .collect::<Vec<_>>()
                    )
                };
                (StatusCode::OK, msg)
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    fmt!("All termination attempts failed: {:?}", errors),
                )
            }
        }
    }
}

/// Api '/app' handler
#[cfg(windows)]
pub async fn handle(Json(_data): Json<QueryData>) -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        "Application control is not implemented on Windows yet",
    )
}
