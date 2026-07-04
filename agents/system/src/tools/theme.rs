use crate::prelude::*;
use system_utils::{SystemTheme, ThemeStyle};

#[derive(Deserialize)]
pub struct ThemeAction {
    style: ThemeStyle,
}

/// API: Handles the `theme` swithing
#[log(skip_all, fields(action))]
pub async fn handle_set_theme(tx: Sender<Bytes>, action: ThemeAction) -> Result<()> {
    match SystemTheme::switch(action.style.clone()).await {
        Ok(_) => {
            let msg = str!("System theme switched into {} mode", action.style);
            info!("{msg}");
            tx.send(Chunk::answer(msg)).await?;
            Ok(())
        }
        Err(e) => Err(str!("Switching system theme failed: {e}").into()),
    }
}
