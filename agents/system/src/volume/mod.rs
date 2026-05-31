pub mod mode;
pub use mode::VolumeMode;

pub mod action;
pub use action::VolumeAction;

pub mod status;
pub use status::VolumeStatus;

use crate::prelude::*;
use tokio::process::Command;

/// The audio volume manager
#[derive(Debug)]
pub struct Volume;

impl Volume {
    #[cfg(target_os = "linux")]
    const MAX_VOLUME: u32 = 200;
    #[cfg(not(target_os = "linux"))]
    const MAX_VOLUME: u32 = 100;

    /// Executes the audio volume action
    pub async fn execute(mode: VolumeMode, value: Option<i32>) -> Result<VolumeStatus> {
        use VolumeMode::*;

        // get current volume:
        let current_vol = Self::get_volume().await?.clamp(0, Self::MAX_VOLUME);

        // execute action command:
        match mode {
            Get => {
                if Self::is_muted().await.unwrap_or(false) {
                    Ok(VolumeStatus::Muted)
                } else {
                    Ok(VolumeStatus::Active {
                        volume: current_vol,
                    })
                }
            }

            Set => {
                let val = value.ok_or_else(|| Error::ExpectedValue(Set))?;
                let target_vol = (val.max(0) as u32).min(Self::MAX_VOLUME);

                if target_vol != current_vol {
                    Self::set_volume(target_vol).await?;
                }
                Ok(VolumeStatus::Active { volume: target_vol })
            }

            Add => {
                let delta = value.ok_or_else(|| Error::ExpectedValue(Add))?;
                let calculated = (current_vol as i32) + delta;
                let target_vol = calculated.clamp(0, Self::MAX_VOLUME as i32) as u32;

                if target_vol != current_vol {
                    Self::set_volume(target_vol).await?;
                }
                Ok(VolumeStatus::Active { volume: target_vol })
            }

            Mute => {
                Self::set_mute(true).await?;
                Ok(VolumeStatus::Muted)
            }

            Unmute => {
                Self::set_mute(false).await?;
                Ok(VolumeStatus::Active {
                    volume: current_vol,
                })
            }
        }
    }

    /// Returns the audio volume [0-100]%
    pub async fn get_volume() -> Result<u32> {
        // --- Linux (PulseAudio / PipeWire) ---
        #[cfg(target_os = "linux")]
        {
            let output = Command::new("pactl")
                .args(["list", "sinks"])
                .output()
                .await
                .map_err(|e| Error::GetVolume(e.into()))?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let re = re!(r"Volume: .*?(\d+)%");

            if let Some(caps) = re.captures(&stdout)
                && let Some(vol_str) = caps.get(1)
            {
                return Ok(vol_str.as_str().parse()?);
            }
            return Err(Error::GetVolume(Error::DevicesNotFound.into()).into());
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
            return Ok(stdout.trim().parse::<u32>()?);
        }

        // --- Windows (PowerShell + Core Audio COM API) ---
        #[cfg(target_os = "windows")]
        {
            let script = "$w=(New-Object -ComObject MMDeviceEnumerator).GetDefaultAudioEndpoint(0,0).AudioEndpointVolume; \
                          [int]($w.GetMasterVolumeLevelScalar() * 100)";

            let output = Command::new("powershell")
                .args(["-Command", &script])
                .output()
                .await
                .map_err(|e| Error::GetVolume(e.into()))?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout
                .trim()
                .parse::<u32>()
                .map_err(|_| Error::GetVolume(Error::DevicesNotFound.into()));
        }
    }

    /// Sets the audio volume [0-100]%
    pub async fn set_volume(vol: u32) -> Result<()> {
        let vol = vol.min(100);

        // --- Linux ---
        #[cfg(target_os = "linux")]
        {
            Command::new("pactl")
                .args(["set-sink-volume", "@DEFAULT_SINK@", &str!("{vol}%")])
                .status()
                .await
                .map_err(|e| Error::SetVolume(e.into()))?;
        }

        // --- MacOS ---
        #[cfg(target_os = "macos")]
        {
            Command::new("osascript")
                .args(["-e", &str!("set volume output volume {vol}")])
                .status()
                .await
                .map_err(|e| Error::SetVolume(e.into()))?;
        }

        // --- Windows ---
        #[cfg(target_os = "windows")]
        {
            let normalized_vol = (vol as f32) / 100.0;
            let script = str!(
                "$w=(New-Object -ComObject MMDeviceEnumerator).GetDefaultAudioEndpoint(0,0).AudioEndpointVolume; \
                 $w.SetMasterVolumeLevelScalar({normalized_vol}, $null)"
            );

            Command::new("powershell")
                .args(["-Command", &script])
                .status()
                .await
                .map_err(|e| Error::SetVolume(e.into()))?;
        }

        Ok(())
    }

    /// Mutes/unmutes the audio volume
    pub async fn set_mute(mute: bool) -> Result<()> {
        // --- Linux ---
        #[cfg(target_os = "linux")]
        {
            let mute_arg = if mute { "1" } else { "0" };
            Command::new("pactl")
                .args(["set-sink-mute", "@DEFAULT_SINK@", mute_arg])
                .status()
                .await
                .map_err(|e| Error::SetMute(e.into()))?;
        }

        // --- MacOS ---
        #[cfg(target_os = "macos")]
        {
            let mute_arg = if mute { "true" } else { "false" };
            Command::new("osascript")
                .args(["-e", &str!("set volume muted {mute_arg}")])
                .status()
                .await
                .map_err(|e| Error::SetMute(e.into()))?;
        }

        // --- Windows ---
        #[cfg(target_os = "windows")]
        {
            let mute_arg = if mute { "$true" } else { "$false" };
            let script = str!(
                "$w=(New-Object -ComObject MMDeviceEnumerator).GetDefaultAudioEndpoint(0,0).AudioEndpointVolume; \
                 $w.Mute = {mute_arg}"
            );

            Command::new("powershell")
                .args(["-Command", &script])
                .status()
                .await
                .map_err(|e| Error::SetMute(e.into()))?;
        }

        Ok(())
    }

    /// Returns true if audio is muted
    pub async fn is_muted() -> Result<bool> {
        // --- Linux ---
        #[cfg(target_os = "linux")]
        {
            let output = Command::new("pactl")
                .args(["get-sink-mute", "@DEFAULT_SINK@"])
                .output()
                .await
                .map_err(|e| Error::GetMute(e.into()))?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.contains("yes"))
        }

        // --- MacOS ---
        #[cfg(target_os = "macos")]
        {
            let output = Command::new("osascript")
                .args(["-e", "output muted of (get volume settings)"])
                .output()
                .await
                .map_err(|e| Error::GetMute(e.into()))?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.trim() == "true")
        }

        // --- Windows ---
        #[cfg(target_os = "windows")]
        {
            let script = "$w=(New-Object -ComObject MMDeviceEnumerator).GetDefaultAudioEndpoint(0,0).AudioEndpointVolume; $w.Mute";
            let output = Command::new("powershell")
                .args(["-Command", &script])
                .output()
                .await
                .map_err(|e| Error::GetMute(e.into()))?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.trim() == "True")
        }
    }
}
