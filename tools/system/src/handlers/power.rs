use crate::prelude::*;
use tokio::process::Command;

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
}

/// The request POST data
#[derive(Deserialize)]
pub struct QueryData {
    mode: PowerMode,
}

/// Api '/power' handler
pub async fn handle(Json(data): Json<QueryData>) -> impl IntoResponse {
    let status = match data.mode {
        PowerMode::TurnOff => {
            warn!("Trying to turn off");

            #[cfg(unix)]
            {
                Command::new("shutdown")
                    .status()
                    .await
                    .map_err(|e| fmt!("Fail with turn off PC: {e}"))
            }
            #[cfg(windows)]
            {
                Command::new("shutdown")
                    .args(&["/s"])
                    .status()
                    .await
                    .map_err(|e| fmt!("Fail with turn off PC: {e}"))
            }
        }

        PowerMode::Sleep => {
            warn!("Trying to sleep");

            #[cfg(unix)]
            {
                Command::new("systemctl")
                    .arg("suspend")
                    .status()
                    .await
                    .map_err(|e| fmt!("Fail with suspend PC: {e}"))
            }
            #[cfg(windows)]
            {
                Command::new("rundll32.exe")
                    .args(&["powrprof.dll,SetSuspendState", "0,1,0"])
                    .status()
                    .await
                    .map_err(|e| error!("Fail with sleep PC: {e}"))
            }
        }

        PowerMode::Reboot => {
            warn!("Trying to reboot..");

            #[cfg(unix)]
            {
                Command::new("reboot")
                    .status()
                    .await
                    .map_err(|e| fmt!("Fail with reboot PC: {e}"))
            }
            #[cfg(windows)]
            {
                Command::new("shutdown")
                    .args(&["/r"])
                    .status()
                    .await
                    .map_err(|e| fmt!("Fail with reboot PC: {e}"))
            }
        }

        PowerMode::Lock => {
            warn!("Trying to lock session..");

            #[cfg(unix)]
            {
                Command::new("loginctl")
                    .arg("lock-session")
                    .status()
                    .await
                    .map_err(|e| fmt!("Fail with lock PC session: {e}"))
            }

            #[cfg(windows)]
            {
                Command::new("rundll32.exe")
                    .args(&["user32.dll,LockWorkStation"])
                    .status()
                    .await
                    .map_err(|e| fmt!("Fail with lock PC session: {e}"))
            }
        }
    };

    match status {
        Ok(_) => (
            StatusCode::OK,
            HeaderMap::from_iter(map! {
                header::CONTENT_TYPE => "text/plain".parse().unwrap()
            }),
            Body::empty(),
        ),
        Err(e) => {
            error!("{e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                HeaderMap::from_iter(map! {
                    header::CONTENT_TYPE => "text/plain".parse().unwrap()
                }),
                Body::new(e),
            )
        }
    }
    .into_response()
}
