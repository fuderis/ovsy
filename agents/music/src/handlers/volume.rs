use crate::prelude::*;
use tokio::process::Command;

/// The request POST data
#[derive(Deserialize)]
pub struct QueryData {
    delta: i32,
    #[serde(default)]
    force: bool,
}

/// Api '/volume' handler
pub async fn handle(Json(data): Json<QueryData>) -> impl IntoResponse {
    // creating HTTP stream body:
    let body = Stream::body(move |tx| async move {
        let mut session = Session::new(tx);

        session.think("Calculating target volume...").await.ok();

        let vol = if !data.force {
            match get_volume().await {
                Ok(vol) => vol as i32 + data.delta,
                Err(e) => {
                    error!("Failed to get volume: {e}");
                    session
                        .error(e.to_string(), "Failed to get current volume")
                        .await
                        .ok();
                    return;
                }
            }
        } else {
            data.delta
        }
        .clamp(0, 100) as u32;

        session
            .think(str!("Setting up the volume to {vol}%..."))
            .await
            .ok();
        info!("Volume set to {vol}%");

        match set_volume(vol).await {
            Ok(_) => {
                let success_msg = str!("Volume set to {vol}%");
                session.info(success_msg).await.ok();
            }
            Err(e) => {
                error!("Failed to set volume: {e}");
                session
                    .error(e.to_string(), "Failed to set volume")
                    .await
                    .ok();
            }
        }
    });

    // send stream to client:
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/octet-stream")],
        Body::from_stream(body),
    )
        .into_response()
}

/// Sets system volume [0-100]%
async fn set_volume(vol: u32) -> Result<()> {
    // clamp the volume to a maximum of 100% to prevent OS-level errors:
    let vol = vol.min(100);

    // --- Linux (PulseAudio / PipeWire) ---
    #[cfg(target_os = "linux")]
    {
        Command::new("pactl")
            .args(["set-sink-volume", "@DEFAULT_SINK@", &str!("{vol}%")])
            .output()
            .await
            .map_err(|e| Error::SetVolume(Box::new(e.into())))?;
    }

    // --- MacOS (AppleScript) ---
    #[cfg(target_os = "macos")]
    {
        // AppleScript uses values from 0 to 7 by default, but 'output volume' accepts 0-100 scale:
        Command::new("osascript")
            .args(["-e", &str!("set volume output volume {vol}")])
            .output()
            .await
            .map_err(|e| Error::SetVolume(Box::new(e)))?;
    }

    // --- Windows (PowerShell + Core Audio COM API) ---
    #[cfg(target_os = "windows")]
    {
        // Windows API expects a scalar value between 0.0 and 1.0
        let normalized_vol = (vol as f32) / 100.0;

        // we fetch the default audio endpoint and set its master volume scalar:
        let script = str!(
            "$w=(New-Object -ComObject MMDeviceEnumerator).GetDefaultAudioEndpoint(0,0).AudioEndpointVolume; \
             $w.SetMasterVolumeLevelScalar({normalized_vol}, $null)"
        );

        Command::new("powershell")
            .args(["-Command", &script])
            .output()
            .await
            .map_err(|e| Error::SetVolume(e))?;
    }

    Ok(())
}

/// Returns system volume [0-100]%
async fn get_volume() -> Result<u32> {
    // --- Linux (PulseAudio / PipeWire) ---
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("pactl")
            .args(["list", "sinks"])
            .output()
            .await
            .map_err(|e| Error::GetVolume(Box::new(e.into())))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // it looks for a pattern like "Volume: ... 50%" and captures the digits:
        let re = re!(r"Volume: .*?(\d+)%");

        // search for the first match in the command output:
        if let Some(caps) = re.captures(&stdout)
            && let Some(vol_str) = caps.get(1)
        {
            return Ok(vol_str.as_str().parse()?);
        }

        // return error if pactl output didn't contain valid volume info:
        return Err(Error::GetVolume(Box::new(Error::DevicesNotFound.into())).into());
    }

    // --- MacOS (AppleScript) ---
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("osascript")
            .args(["-e", "output volume of (get volume settings)"])
            .output()
            .await
            .map_err(|e| Error::GetVolume(e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // trim whitespace/newlines and parse the plain number string:
        let vol = stdout.trim().parse::<u32>()?;

        return Ok(vol);
    }

    // --- Windows (PowerShell + Core Audio COM API) ---
    #[cfg(target_os = "windows")]
    {
        // get the scalar volume (0.0 to 1.0), multiply by 100, and cast to Int:
        let script = "$w=(New-Object -ComObject MMDeviceEnumerator).GetDefaultAudioEndpoint(0,0).AudioEndpointVolume; \
                      [int]($w.GetMasterVolumeLevelScalar() * 100)";

        let output = Command::new("powershell")
            .args(["-Command", &script])
            .output()
            .await
            .map_err(|e| Error::Custom(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let vol_str = stdout.trim();

        // parse the resulting string into u32:
        return vol_str
            .parse::<u32>()
            .map_err(|_| Error::GetVolume(Box::new(Error::DevicesNotFound.into())));
    }
}
