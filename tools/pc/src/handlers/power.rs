use crate::prelude::*;
use tokio::process::Command;
// use tokio::fs as tfs;

static CANCEL_OPERATION: Flag = Flag::new();
const DEFAULT_TIMEOUT: u64 = 3;

/// The power mode
#[derive(Display, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PowerMode {
    #[serde(rename = "off")]
    #[display = "turn off"]
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

/// Api '/play' handler
pub async fn handle(Json(data): Json<QueryData>) -> Json<JsonValue> {    
    if let PowerMode::Cancel = &data.mode {
        CANCEL_OPERATION.set(true);
        return Json(json!({ "status": 200 }));
    } else {
        CANCEL_OPERATION.set(false);

        match &data.mode {
            PowerMode::TurnOff => warn!("Turn off after {} sec..", data.timeout),
            PowerMode::Sleep   => warn!("Sleep after {} sec..", data.timeout),
            PowerMode::Reboot  => warn!("Reboot after {} sec..", data.timeout),
            PowerMode::Lock    => warn!("Lock after {} sec..", data.timeout),
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
                warn!("Power operation '{}' canceled.", data.mode);
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
                let _ = Command::new("shutdown")
                    .args(&["/s"])
                    .status()
                    .await
                    .map_err(|e| err!("Fail with turn off PC: {e}"));
            }
    
            PowerMode::Sleep => {
                let _ = Command::new("rundll32.exe")
                    .args(&["powrprof.dll,SetSuspendState", "0,1,0"])
                    .status()
                    .await
                    .map_err(|e| err!("Fail with sleep PC: {e}"));
            }
    
            PowerMode::Reboot => {
                let _ = Command::new("shutdown")
                    .args(&["/r"])
                    .status()
                    .await
                    .map_err(|e| err!("Fail with reboot PC: {e}"));
            }
    
            PowerMode::Lock => {
                let _ = Command::new("rundll32.exe")
                    .args(&["user32.dll,LockWorkStation"])
                    .status()
                    .await
                    .map_err(|e| err!("Fail with lock PC: {e}"));
            }
    
            _ => {}
        }
    });
    
    Json(json!({ "status": 200 }))
}
