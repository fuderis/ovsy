use crate::{prelude::*, theme::*};

/// API: Handles the `theme` tool
#[log(skip_all, fields(action))]
pub async fn handle_theme(tx: Arc<StreamSender<Bytes>>, action: ThemeAction) -> Result<()> {
    match Theme::execute(action.clone()).await {
        Ok(_) => {
            let msg = str!("System swithed into {} theme", action.mode);
            info!("{msg}");
            tx.send(Chunk::answer(msg))?;
            Ok(())
        }
        Err(e) => {
            error!("Switch system theme failed: {e}");
            Err(str!("Switch system theme failed: {e}").into())
        }
    }
}
