use crate::{prelude::*, volume::*};

/// API: Handles the `volume` tool
pub async fn handle(tx: Arc<StreamSender<Bytes>>, data: JsonValue) -> Result<()> {
    let action: VolumeAction = json::from_value(data)?;

    match Volume::execute(action.mode, action.value).await {
        Ok(status) => {
            use VolumeMode::*;
            use VolumeStatus::*;

            let msg = match (action.mode, status) {
                (_, Muted) => str!("Audio is muted"),
                (Get, Active { volume }) => str!("Current volume is {volume}%"),
                (Mute | Unmute, Active { volume }) => {
                    str!("Audio is unmuted (Volume: {volume}%)")
                }
                (Set | Add, Active { volume }) => str!("Volume updated to {volume}%"),
            };

            info!("{msg}");
            tx.send(Chunk::answer(msg))?;
            Ok(())
        }
        Err(e) => {
            error!("Failed to process volume action: {e}");
            Err(str!("Failed to process volume action: {e:?}").into())
        }
    }
}
