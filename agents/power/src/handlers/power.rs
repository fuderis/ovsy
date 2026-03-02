use crate::{
    power::{DEFERRED_POWER_ACTION, PowerMode},
    prelude::*,
};
use tokio::process::Command;

/// The request POST data
#[derive(Deserialize)]
pub struct QueryData {
    mode: PowerMode,
    #[serde(default)]
    timeout: u64,
}

/// Api '/power' handler
pub async fn handle(Json(data): Json<QueryData>) -> impl IntoResponse {
    let QueryData { mode, timeout } = data;

    match defer_power(mode.clone(), timeout).await {
        Ok(_) => {
            info!("Deferred {mode} for {timeout} seconds..");

            (
                StatusCode::OK,
                HeaderMap::from_iter(map! {
                    header::CONTENT_TYPE => "text/plain".parse().unwrap()
                }),
                Body::new(fmt!(
                    "[Success] Power operation {mode} will be executed in {timeout} seconds.."
                )),
            )
        }
        Err(e) => {
            error!("{e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                HeaderMap::from_iter(map! {
                    header::CONTENT_TYPE => "text/plain".parse().unwrap()
                }),
                Body::new(fmt!("[Error] {e}")),
            )
        }
    }
    .into_response()
}

/// Deferres power operation
async fn defer_power(mode: PowerMode, timeout: u64) -> Result<()> {
    // cancel old power action:
    if let Some(_) = DEFERRED_POWER_ACTION.lock().await.take() {
        sleep(Duration::from_millis(2000)).await;
    }
    // set new power action:
    let _ = DEFERRED_POWER_ACTION.lock().await.insert(mode.clone());

    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(1));
        let start_time = Instant::now();
        let end_time = Duration::from_secs(timeout);

        loop {
            interval.tick().await;

            // check for canceled:
            if Some(&mode) != DEFERRED_POWER_ACTION.lock().await.as_ref() {
                return;
            }

            // check elapsed time:
            if start_time.elapsed() >= end_time {
                break;
            }
        }

        // do power action:
        power(mode).await;
    });

    Ok(())
}

/// Do power operation
async fn power(mode: PowerMode) {
    warn!("Trying to {mode} system..");

    let status = match mode {
        PowerMode::TurnOff => {
            #[cfg(unix)]
            {
                Command::new("shutdown").status().await
            }
            #[cfg(windows)]
            {
                Command::new("shutdown").args(&["/s"]).status().await
            }
        }

        PowerMode::Suspend => {
            #[cfg(unix)]
            {
                Command::new("systemctl").arg("suspend").status().await
            }
            #[cfg(windows)]
            {
                Command::new("rundll32.exe")
                    .args(&["powrprof.dll,SetSuspendState", "0,1,0"])
                    .status()
                    .await
            }
        }

        PowerMode::Reboot => {
            #[cfg(unix)]
            {
                Command::new("reboot").status().await
            }
            #[cfg(windows)]
            {
                Command::new("shutdown").args(&["/r"]).status().await
            }
        }

        PowerMode::Lock => {
            #[cfg(unix)]
            {
                Command::new("loginctl").arg("lock-session").status().await
            }
            #[cfg(windows)]
            {
                Command::new("rundll32.exe")
                    .args(&["user32.dll,LockWorkStation"])
                    .status()
                    .await
            }
        }
    };

    if let Err(e) = status {
        error!("Fail with {mode} system: {e}");
    }
}
