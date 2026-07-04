use crate::prelude::*;
use anylm::{Schema, Tool};
use system_utils::{SystemTheme, ThemeStyle};

pub fn theme_switcher_tools() -> Vec<Tool> {
    vec![
        Tool::new("set_theme", "Changes the system appearance theme.").required_property(
            "style",
            Schema::string("Target theme style.").variants(set![str!("light"), str!("dark"),]),
        ),
    ]
}

#[derive(Deserialize)]
pub struct ThemeAction {
    style: ThemeStyle,
}

#[log(skip_all, fields(action))]
pub async fn handle_set_theme(tx: Sender<Bytes>, action: ThemeAction) -> Result<()> {
    match SystemTheme::switch(action.style.clone()).await {
        Ok(_) => {
            let msg = str!("System theme switched into {} mode", action.style);
            info!("{msg}");
            tx.send(Chunk::answer(msg))?;
            Ok(())
        }
        Err(e) => Err(str!("Switching system theme failed: {e}").into()),
    }
}
