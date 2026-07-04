use crate::prelude::*;
use system_utils::AudioControl;

#[derive(Deserialize)]
pub struct SetVolumeAction {
    volume: u32,
}

#[derive(Deserialize)]
pub struct DeltaVolumeAction {
    amount: u32,
}

/// API: Sets the audio volume
#[log(skip_all, fields(action))]
pub async fn handle_set_volume(tx: Sender<Bytes>, action: SetVolumeAction) -> Result<()> {
    match AudioControl::set_volume(action.volume as u32).await {
        Ok(_) => {
            let msg = str!(
                "Audio volume updated successfully. Current volume: {}%.",
                action.volume
            );
            info!("{msg}");
            tx.send(Chunk::answer(msg)).await?;
            Ok(())
        }
        Err(e) => Err(str!("Failed to update audio volume: {e:?}").into()),
    }
}

/// API: Increases the audio volume
#[log(skip_all, fields(action))]
pub async fn handle_increase_volume(tx: Sender<Bytes>, action: DeltaVolumeAction) -> Result<()> {
    match AudioControl::increase_volume(action.amount).await {
        Ok(volume) => {
            let msg = str!("Audio volume increased successfully. Current volume: {volume}%.",);
            info!("{msg}");
            tx.send(Chunk::answer(msg)).await?;
            Ok(())
        }
        Err(e) => Err(str!("Failed to update audio volume: {e:?}").into()),
    }
}

/// API: Decreases the audio volume
#[log(skip_all, fields(action))]
pub async fn handle_decrease_volume(tx: Sender<Bytes>, action: DeltaVolumeAction) -> Result<()> {
    match AudioControl::decrease_volume(action.amount).await {
        Ok(volume) => {
            let msg = str!("Audio volume decreased successfully. Current volume: {volume}%.",);
            info!("{msg}");
            tx.send(Chunk::answer(msg)).await?;
            Ok(())
        }
        Err(e) => Err(str!("Failed to update audio volume: {e:?}").into()),
    }
}

/// API: Returns the current audio volume
#[log(skip_all)]
pub async fn handle_get_volume(tx: Sender<Bytes>) -> Result<()> {
    match AudioControl::get_volume().await {
        Ok(volume) => {
            let msg = str!("Current audio volume {volume}%.");
            info!("{msg}");
            tx.send(Chunk::answer(msg)).await?;
            Ok(())
        }
        Err(e) => Err(str!("Failed to get audio volume: {e:?}").into()),
    }
}

#[derive(Deserialize)]
pub struct MuteAction {
    mute: bool,
}

/// API: Includes/Turns off the sound
#[log(skip_all, fields(action))]
pub async fn handle_set_mute(tx: Sender<Bytes>, action: MuteAction) -> Result<()> {
    match AudioControl::set_mute(action.mute).await {
        Ok(_) => {
            let msg = if action.mute {
                "Audio muted successfully."
            } else {
                "Audio unmuted successfully."
            };
            info!("{msg}");
            tx.send(Chunk::answer(msg)).await?;
            Ok(())
        }
        Err(e) => Err(str!("Failed to update audio mute state: {e:?}").into()),
    }
}

/// API: Returns the audio mute status
#[log(skip_all)]
pub async fn handle_is_muted(tx: Sender<Bytes>) -> Result<()> {
    match AudioControl::is_muted().await {
        Ok(is_muted) => {
            let msg = if is_muted {
                "Audio is currently muted."
            } else {
                "Audio is currently unmuted."
            };
            info!("{msg}");
            tx.send(Chunk::answer(msg)).await?;
            Ok(())
        }
        Err(e) => Err(str!("Failed to get audio mute state: {e:?}").into()),
    }
}
