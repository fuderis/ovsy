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

    // creating HTTP stream body:
    let body = Stream::body(move |tx| async move {
        let mut session = Session::new(tx);

        session
            .think(fmt!("Preparing power operation '{mode}'.."))
            .await
            .ok();

        // cancel old power action:
        if DEFERRED_POWER_ACTION.lock().await.take().is_some() {
            session
                .think("Canceling previous pending power action...")
                .await
                .ok();
            sleep(Duration::from_millis(2000)).await;
        }

        // set new power action:
        let _ = DEFERRED_POWER_ACTION.lock().await.insert(mode.clone());
        info!("Deferred {mode} for {timeout} seconds..");

        // streaming the successful response BEFORE the countdown starts and end the stream:
        session
            .info(fmt!(
                "Power operation {mode} scheduled in {timeout} seconds."
            ))
            .await
            .ok();

        // generating a background task in Tokio runtime:
        // (it will continue to live after this stream ends..)
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));
            let start_time = Instant::now();
            let end_time = Duration::from_secs(timeout);

            loop {
                interval.tick().await;

                // check for canceled (если прилетит новый запрос на /power):
                if Some(&mode) != DEFERRED_POWER_ACTION.lock().await.as_ref() {
                    info!("Power action '{mode}' was canceled.");
                    return;
                }

                if start_time.elapsed() >= end_time {
                    break;
                }
            }

            // clean up the global state before execution:
            let _ = DEFERRED_POWER_ACTION.lock().await.take();

            // do power action:
            power(mode).await;
        });
    });

    // send stream to client:
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/octet-stream")],
        Body::from_stream(body),
    )
        .into_response()
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
