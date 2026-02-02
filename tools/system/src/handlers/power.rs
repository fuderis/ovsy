use crate::prelude::*;
use tokio::process::Command;
// use tokio::fs as tfs;

static CANCEL_OPERATION: Flag = Flag::new();
const DEFAULT_TIMEOUT: u64 = 3;

/// The power mode
#[derive(Display, Serialize, Deserialize)]
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
pub async fn handle(Json(data): Json<QueryData>) -> Json<JsonValue> {
    if let PowerMode::Cancel = &data.mode {
        CANCEL_OPERATION.set(true);
        return Json(json!({ "status": 200 }));
    } else {
        CANCEL_OPERATION.set(false);

        match &data.mode {
            PowerMode::TurnOff => warn!("Turn off after {} sec..", data.timeout),
            PowerMode::Sleep => warn!("Sleep after {} sec..", data.timeout),
            PowerMode::Reboot => warn!("Reboot after {} sec..", data.timeout),
            PowerMode::Lock => warn!("Lock after {} sec..", data.timeout),
            _ => {}
        }
    }

    tokio::spawn(async move {
        // init timer:
        let timer = Instant::now();
        let timeout = Duration::from_secs(data.timeout);

        // wait timer:
        loop {
            if CANCEL_OPERATION.is_true() {
                warn!("Power operation '{}' canceled", data.mode);
                return;
            }

            // check timer:
            if timer.elapsed() >= timeout {
                break;
            }

            sleep(Duration::from_millis(1000)).await;
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
                        .map_err(|e| err!("Fail with turn off PC: {e}"));
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
                        .map_err(|e| err!("Fail with sleep PC: {e}"));
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
                        .map_err(|e| err!("Fail with reboot PC: {e}"));
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
                        .map_err(|e| err!("Fail with lock PC session: {e}"));
                }
            }

            _ => {}
        }
    });

    Json(json!({ "status": 200 }))
}
