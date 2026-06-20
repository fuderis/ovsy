pub mod mode;
pub use mode::ThemeMode;

pub mod action;
pub use action::ThemeAction;

use crate::prelude::*;

/// The system theme manager
#[derive(Debug)]
pub struct Theme;

impl Theme {
    /// Executes the theme action
    pub async fn execute(action: ThemeAction) -> Result<()> {
        let ThemeAction { mode } = action;
        let is_dark = mode == ThemeMode::Dark;

        Self::switch(is_dark).await?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    async fn switch(dark: bool) -> Result<()> {
        use tokio::process::Command;

        let schema = if dark { "prefer-dark" } else { "prefer-light" };

        let status = Command::new("gsettings")
            .args(&["set", "org.gnome.desktop.interface", "color-scheme", schema])
            .status()
            .await
            .map_err(Error::GsettingsExecute)?;

        if !status.success() {
            return Err(Error::GsettingsExitStatus.into());
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    async fn switch(dark: bool) -> Result<()> {
        use tokio::process::Command;

        let script = format!(
            "tell application \"System Events\" to tell appearance preferences to set dark mode to {}",
            dark
        );

        let status = Command::new("osascript")
            .args(&["-e", &script])
            .status()
            .await
            .map_err(Error::OsascriptExecute)?;

        if !status.success() {
            return Err(Error::OsascriptExitStatus);
        }

        Ok(())
    }

    #[cfg(target_os = "windows")]
    async fn switch(dark: bool) -> Result<(), Error> {
        use winreg::RegKey;
        use winreg::enums::*;

        tokio::task::spawn_blocking(move || {
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            let path = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
            let key = hkcu.open_subkey_with_flags(path, KEY_SET_VALUE)?;

            let val = if dark { 0u32 } else { 1u32 };
            key.set_value("AppsUseLightTheme", &val)?;
            key.set_value("SystemUsesLightTheme", &val)?;
            Ok::<(), std::io::Error>(())
        })
        .await
        .map_err(|e| Error::TaskJoin(e.into()))?
        .map_err(Error::Registry)?;

        Ok(())
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    async fn switch(_dark: bool) -> Result<()> {
        Err(Error::UnsupportedOS)
    }
}
