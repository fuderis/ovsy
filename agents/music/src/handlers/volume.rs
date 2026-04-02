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
            .think(fmt!("Setting the volume to {vol}%..."))
            .await
            .ok();
        info!("Volume set to {vol}%");

        match set_volume(vol).await {
            Ok(_) => {
                let success_msg = fmt!("Volume set to {vol}%");
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

/// Sets system volume
async fn set_volume(vol: u32) -> Result<()> {
    #[cfg(unix)]
    {
        Command::new("pactl")
            .args(["set-sink-volume", "@DEFAULT_SINK@", &fmt!("{vol}%")])
            .output()
            .await
            .map_err(|e| Error::PactlError(e))?;
    }

    #[cfg(windows)]
    {
        unimplemented!();
    }

    Ok(())
}

/// Returns system volume
async fn get_volume() -> Result<u32> {
    #[cfg(unix)]
    {
        let output = Command::new("pactl")
            .args(["list", "sinks"])
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let re = re!(r#"Volume: .*(\d+)%"#);

        // parsing first line:
        if let Some(caps) = re.captures(&stdout) {
            if let Some(vol_str) = caps[0].split_whitespace().last() {
                if let Some(vol_percent) = vol_str.strip_suffix('%') {
                    return Ok(vol_percent.parse()?);
                }
            }
        }

        Err(Error::DevicesNotFound.into())
    }

    #[cfg(windows)]
    {
        unimplemented!();
    }
}
