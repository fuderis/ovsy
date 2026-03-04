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
    let vol = if !data.force {
        match get_volume().await {
            Ok(vol) => vol as i32 + data.delta,
            Err(e) => {
                error!("Failed to get volume: {e}");
                return (StatusCode::INTERNAL_SERVER_ERROR, fmt!("[Error] {e}")).into_response();
            }
        }
    } else {
        data.delta
    }
    .clamp(0, 100) as u32;

    info!("Set volume to {vol}%");
    match set_volume(vol).await {
        Ok(_) => {
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "text/plain".parse().unwrap());
            (
                StatusCode::OK,
                headers,
                Body::new(fmt!("[Success] Set volume to {vol}%")),
            )
                .into_response()
        }
        Err(e) => {
            error!("Failed to set volume: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, fmt!("[Error] {e}")).into_response()
        }
    }
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
