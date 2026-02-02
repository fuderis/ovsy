use crate::prelude::*;
use tokio::process::Command;

/// The active operation type
static POWER_OPERATION: State<Option<(PowerMode, &'static str)>> = State::new();
/// Default timeout before power off
const DEFAULT_TIMEOUT: u64 = 3;

/// The power mode
#[derive(Clone, Copy, Display, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PowerMode {
    #[serde(rename = "turnoff")]
    #[display = "turnoff"]
    TurnOff,
    #[display = "sleep"]
    Sleep,
    #[display = "reboot"]
    Reboot,
    #[display = "lock"]
    Lock,
    #[display = "cancel"]
    Cancel,
}

/// The request POST data
#[derive(Deserialize)]
pub struct QueryData {
    mode: PowerMode,
    #[serde(default = "QueryData::timeout_default")]
    timeout: u64,
}

impl QueryData {
    pub fn timeout_default() -> u64 {
        DEFAULT_TIMEOUT
    }
}

/// Api '/power' handler
pub async fn handle(Json(data): Json<QueryData>) -> impl IntoResponse {
    // cancel previews operation:
    if let Some((_mode, oper_name)) = POWER_OPERATION.lock().await.take() {
        sleep(Duration::from_secs(1)).await;

        if let PowerMode::Cancel = &data.mode {
            return (
                StatusCode::OK,
                HeaderMap::from_iter(map!(
                    header::CONTENT_TYPE =>
                    "text/plain".parse().unwrap(),
                )),
                Body::new(str!("{oper_name} is canceled")),
            )
                .into_response();
        }
    }

    // planning new operation:
    let oper_name = match data.mode {
        PowerMode::TurnOff => "Turn off",
        PowerMode::Sleep => "Sleep",
        PowerMode::Reboot => "Reboot",
        PowerMode::Lock => "Lock session",
        _ => unreachable!(),
    };
    POWER_OPERATION.set(Some((data.mode, oper_name))).await;

    tokio::spawn(async move {
        // init timer:
        let timer = Instant::now();
        let timeout = Duration::from_secs(data.timeout);
        let mut interval = interval(Duration::from_secs(1));

        // wait timer:
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let elapsed = timer.elapsed();
                    if elapsed >= timeout {
                        break;
                    }

                    // Безопасная проверка состояния
                    match POWER_OPERATION.lock().await.as_ref() {
                        Some((mode, _)) if *mode == data.mode => continue,
                        _ => {
                            warn!("{oper_name} canceled");
                            return;
                        }
                    }
                }
            }
        }

        // do action:
        match data.mode {
            PowerMode::TurnOff => {
                #[cfg(unix)]
                {
                    let _ = Command::new("shutdown")
                        .status()
                        .await
                        .map_err(|e| error!("Fail with turn off PC: {e}"));
                }
                #[cfg(windows)]
                {
                    let _ = Command::new("shutdown")
                        .args(&["/s"])
                        .status()
                        .await
                        .map_err(|e| error!("Fail with turn off PC: {e}"));
                }
            }

            PowerMode::Sleep => {
                #[cfg(unix)]
                {
                    let _ = Command::new("systemctl")
                        .arg("suspend")
                        .status()
                        .await
                        .map_err(|e| error!("Fail with suspend PC: {e}"));
                }
                #[cfg(windows)]
                {
                    let _ = Command::new("rundll32.exe")
                        .args(&["powrprof.dll,SetSuspendState", "0,1,0"])
                        .status()
                        .await
                        .map_err(|e| error!("Fail with sleep PC: {e}"));
                }
            }

            PowerMode::Reboot => {
                #[cfg(unix)]
                {
                    let _ = Command::new("reboot")
                        .status()
                        .await
                        .map_err(|e| error!("Fail with reboot PC: {e}"));
                }
                #[cfg(windows)]
                {
                    let _ = Command::new("shutdown")
                        .args(&["/r"])
                        .status()
                        .await
                        .map_err(|e| error!("Fail with reboot PC: {e}"));
                }
            }

            PowerMode::Lock => {
                #[cfg(unix)]
                {
                    let _ = Command::new("loginctl")
                        .arg("lock-session")
                        .status()
                        .await
                        .map_err(|e| error!("Fail with lock PC session: {e}"));
                }

                #[cfg(windows)]
                {
                    let _ = Command::new("rundll32.exe")
                        .args(&["user32.dll,LockWorkStation"])
                        .status()
                        .await
                        .map_err(|e| error!("Fail with lock PC session: {e}"));
                }
            }

            _ => {}
        }
    });

    // return OK:
    let msg = fmt!("{oper_name} is planned after {} seconds", data.timeout);
    warn!("{msg}");
    (
        StatusCode::OK,
        HeaderMap::from_iter(map! {
            header::CONTENT_TYPE => "text/plain".parse().unwrap()
        }),
        Body::new(msg),
    )
        .into_response()
}
