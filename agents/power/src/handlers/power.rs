use crate::{ACTIVE_ACTION, PowerMode, PowerOptions, prelude::*};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::{
    process::Command,
    time::{Instant, interval},
};

/// API: Handles the `/tool/power` action
pub async fn handle(data: Json<PowerOptions>) -> Response {
    let PowerOptions { mode, timeout } = data.0;

    let body = Stream::body(async move |tx| {
        let now = SystemTime::now();
        let epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
        let timestamp = epoch.as_millis();

        // initiate operation:
        ACTIVE_ACTION.set(Some((timestamp, mode.clone()))).await;

        let msg = str!("Deferred {mode} for {timeout} seconds...");
        info!("{msg}");
        tx.send(Chunk::answer(msg)).ok();

        // start timer loop:
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));
            let start_time = Instant::now();
            let end_time = Duration::from_secs(timeout);

            loop {
                // check for canceled:
                if ACTIVE_ACTION
                    .get()
                    .await
                    .as_ref()
                    .map(|(t, m)| t != timestamp || m != mode)
                    .unwrap_or(true)
                {
                    info!("Power action '{mode}' was canceled.");
                    return;
                }

                if start_time.elapsed() >= end_time {
                    break;
                }

                interval.tick().await;
            }

            // clean up state before execution:
            let _ = ACTIVE_ACTION.set(None).await;

            // do power action:
            shutdown(mode).await;
        });
    });

    Response::ok().stream(body)
}

/// Do poweroff operation
async fn shutdown(mode: PowerMode) {
    warn!("Trying to {mode} system...");

    let (cmd, args): (&str, &[&str]) = match mode {
        PowerMode::Shutdown => {
            #[cfg(windows)]
            {
                ("shutdown", &["/s"])
            }
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            {
                ("shutdown", &["-h", "now"])
            }
        }
        PowerMode::Suspend => {
            #[cfg(target_os = "linux")]
            {
                ("systemctl", &["suspend"])
            }
            #[cfg(target_os = "macos")]
            {
                ("pmset", &["sleepnow"])
            }
            #[cfg(windows)]
            {
                ("rundll32.exe", &["powrprof.dll,SetSuspendState", "0,1,0"])
            }
        }
        PowerMode::Reboot => {
            #[cfg(target_os = "linux")]
            {
                ("reboot", &[])
            }
            #[cfg(target_os = "macos")]
            {
                ("shutdown", &["-r", "now"])
            }
            #[cfg(windows)]
            {
                ("shutdown", &["/r"])
            }
        }
        PowerMode::Lock => {
            #[cfg(target_os = "linux")]
            {
                ("loginctl", &["lock-session"])
            }
            #[cfg(target_os = "macos")]
            {
                ("open", &["-a", "loginwindow"])
            }
            #[cfg(windows)]
            {
                ("rundll32.exe", &["user32.dll,LockWorkStation"])
            }
        }
    };

    if let Err(e) = Command::new(cmd).args(args).status().await {
        error!("Failed to {mode} system: {e}");
    }
}
