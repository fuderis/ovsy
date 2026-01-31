use crate::prelude::*;
use super::Action;
use tokio::process::Command;
use pc_remote::input::{ Keyboard, Key };

/// The IR-code bind
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bind {
    pub codes: Vec<String>,
    pub action: Action,
}

impl Bind {
    /// Handles IR-code
    pub async fn handle(&self, _code: &str, repeat: bool, millis_diff: i64) -> Result<()> {
        if repeat && millis_diff < 20 { return Ok(()); };
        
        match self.action {
            // Media:
            Action::PlayPause      => if !repeat { press_keys(&[Key::PlayPause], 1)? }
            Action::Stop           => if !repeat { press_keys(&[Key::Stop], 1)? }
            Action::VolumeUp       => press_keys(&[Key::VolumeUp], if repeat { 2 }else{ 1 })?,
            Action::VolumeDown     => press_keys(&[Key::VolumeDown], if repeat { 2 }else{ 1 })?,
            Action::Mute           => if !repeat { press_keys(&[Key::Mute], 1)? }
            Action::NextTrack      => if !repeat { press_keys(&[Key::Next], 1)? }
            Action::PrevTrack      => if !repeat { press_keys(&[Key::Prev], 1)? }
            Action::ScrollLeft     => if !repeat { press_keys(&[Key::Left], 1)? }
            Action::ScrollRight    => if !repeat { press_keys(&[Key::Right], 1)? }

            // Workspace:
            Action::TabNext        => if !repeat { press_keys(&[Key::Ctrl, Key::Tab], 1)? }
            Action::TabPrev        => if !repeat { press_keys(&[Key::Ctrl, Key::Shift, Key::Tab], 1)? }
            Action::WinTabNext     => if !repeat { press_keys(&[Key::Alt, Key::Tab], 1)? }
            Action::WinTabPrev     => if !repeat { press_keys(&[Key::Alt, Key::Shift, Key::Tab], 1)? }
            Action::WinNextSpace   => press_keys(&[Key::Ctrl, Key::Super, Key::Right], if repeat { 2 }else{ 1 })?,
            Action::WinPrevSpace   => press_keys(&[Key::Ctrl, Key::Super, Key::Left], if repeat { 2 }else{ 1 })?,

            // Power:
            Action::PowerOff       => if !repeat { power_off(false).await? }
            Action::Sleep          => if !repeat { power_off(true).await? }
        }

        Ok(())
    }
}


/// Do press keyboard hotkey
pub fn press_keys(hotkey: &[Key], times: usize) -> Result<()> {
    let mut keyboard = Keyboard::new()?;

    for _ in 0..times {
        keyboard.press(&hotkey)?;
        keyboard.release(&hotkey)?;
    }

    Ok(())
}

/// Powers off PC
pub async fn power_off(sleep: bool) -> Result<()> {
    // hibernate:
    if !sleep {
        let _ = Command::new("shutdown")
            .args(&["/h"])
            .status()
            .await
            .map_err(|e| fmt!("Hibernate failed: {e}"))?;
    }
    // sleep:
    else {
        let _ = Command::new("rundll32.exe")
            .args(&["powrprof.dll,SetSuspendState", "0,1,0"])
            .status()
            .await
            .map_err(|e| fmt!("Sleep failed: {e}"))?;
    }

    Ok(())
}
