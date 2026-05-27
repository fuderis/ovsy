use crate::{PowerAction, PowerMode, prelude::*};
use tokio::{
    process::Command,
    time::{Instant, interval},
};

/// API: Handles the `/tool/power` action
pub async fn handle(data: Json<PowerAction>) -> Response {
    let action = PowerAction::new_from(data.0);

    let body = Stream::body(async move |tx| {
        // initiate operation:
        PowerAction::set(action.clone()).await;

        let msg = str!("Deferred {} for {} seconds...", action.mode, action.timeout);
        info!("{msg}");
        tx.send(Chunk::answer(msg)).ok();

        // start timer loop:
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));
            let start_time = Instant::now();
            let end_time = Duration::from_secs(action.timeout);

            loop {
                // check for canceled:
                if PowerAction::get()
                    .await
                    .as_ref()
                    .as_ref()
                    .map(|active| {
                        active.timestamp != action.timestamp || active.mode != action.mode
                    })
                    .unwrap_or(true)
                {
                    info!("Power action '{}' was canceled.", &action.mode);
                    return;
                }

                if start_time.elapsed() >= end_time {
                    break;
                }

                interval.tick().await;
            }

            // clean up state before execution:
            let _ = PowerAction::take().await;

            // do power action:
            shutdown(action.mode).await;
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
