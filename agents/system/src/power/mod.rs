pub mod mode;
pub use mode::PowerMode;

pub mod action;
pub use action::PowerAction;

pub mod status;
pub use status::PowerStatus;

use crate::prelude::*;
use tokio::{process::Command, time};

/// The system power manager
#[derive(Debug)]
pub struct Power;

impl Power {
    /// Executes the power operation
    pub async fn execute(action: PowerAction) -> Result<PowerStatus> {
        use PowerMode::*;

        match action.mode {
            Cancel => {
                if let Some(canceled_action) = PowerAction::take().await {
                    Ok(PowerStatus::Canceled {
                        mode: canceled_action.mode,
                    })
                } else {
                    Ok(PowerStatus::NoActiveTask)
                }
            }

            Status => {
                if let Some(active) = &*PowerAction::get().await {
                    let elapsed = active.created_at.elapsed().as_secs();

                    let total_timeout = if let Some(ref t) = active.timeout {
                        t.to_seconds()
                    } else if let Some(target_time) = active.timestamp {
                        (target_time - Utc::now()).num_seconds().max(0) as u64
                    } else {
                        0
                    };

                    let remaining = total_timeout.saturating_sub(elapsed);
                    let target_time = active.timestamp.unwrap_or_else(|| {
                        Utc::now() + chrono::Duration::seconds(remaining as i64)
                    });

                    Ok(PowerStatus::ActiveTask {
                        mode: active.mode,
                        target_time,
                        remaining_secs: remaining,
                    })
                } else {
                    Ok(PowerStatus::NoActiveTask)
                }
            }

            exec_mode => {
                let now = Utc::now();

                // take target time:
                let target_time = if let Some(ref t) = action.timeout {
                    Some(now + chrono::Duration::seconds(t.to_seconds() as i64))
                } else if let Some(target_time) = action.timestamp {
                    if target_time > now {
                        Some(target_time)
                    } else {
                        return Err(str!("Invalid timestamp: requested time is in the past").into());
                    }
                } else {
                    None
                };

                // calculate target time as seconds:
                let timeout_secs = match target_time {
                    Some(target) => (target - now).num_seconds().max(0) as u64,
                    None => 0,
                };

                if timeout_secs == 0 {
                    Self::run_command(exec_mode).await?;
                    Ok(PowerStatus::Executed)
                } else {
                    let final_target = target_time.unwrap();

                    let mut action_clone = action;
                    action_clone.timestamp = Some(final_target);

                    PowerAction::set(action_clone.clone()).await;

                    tokio::spawn(async move {
                        let mut interval = time::interval(Duration::from_secs(1));
                        let start_time = Instant::now();
                        let end_time = Duration::from_secs(timeout_secs);

                        loop {
                            let is_canceled = if let Some(active) = &*PowerAction::get().await {
                                active.created_at != action_clone.created_at
                                    || active.mode != action_clone.mode
                            } else {
                                true
                            };

                            if is_canceled {
                                info!(
                                    "Power action '{}' task was canceled or replaced.",
                                    exec_mode
                                );
                                return;
                            }

                            if start_time.elapsed() >= end_time {
                                break;
                            }

                            interval.tick().await;
                        }

                        let _ = PowerAction::take().await;

                        if let Err(e) = Self::run_command(exec_mode).await {
                            error!(
                                "Failed to execute deferred power action '{}': {e}",
                                exec_mode
                            );
                        }
                    });

                    Ok(PowerStatus::Deferred {
                        mode: exec_mode,
                        target_time: final_target,
                        remaining_secs: timeout_secs,
                    })
                }
            }
        }
    }

    /// Helper function to execute command
    async fn run_command(mode: PowerMode) -> Result<()> {
        use PowerMode::*;
        match mode {
            Shutdown => Self::shutdown().await,
            Reboot => Self::reboot().await,
            Suspend => Self::suspend().await,
            Lock => Self::lock().await,
            Logout => Self::logout().await,
            Cancel | Status => Ok(()),
        }
    }

    /// Does shutdown the system
    pub async fn shutdown() -> Result<()> {
        let (cmd, args): (&str, &[&str]) = {
            #[cfg(windows)]
            {
                ("shutdown", &["/s"])
            }
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            {
                ("shutdown", &["-h", "now"])
            }
        };

        Command::new(cmd).args(args).status().await?;
        Ok(())
    }

    /// Does reboot the system
    pub async fn reboot() -> Result<()> {
        let (cmd, args): (&str, &[&str]) = {
            #[cfg(target_os = "linux")]
            {
                ("reboot", &[])
            }
            #[cfg(target_os = "macos")]
            {
                ("shutdown", &["-r", "now"])
            }
            #[cfg(windows)]
            {
                ("shutdown", &["/r"])
            }
        };

        Command::new(cmd).args(args).status().await?;
        Ok(())
    }

    /// Does suspend the system
    pub async fn suspend() -> Result<()> {
        let (cmd, args): (&str, &[&str]) = {
            #[cfg(target_os = "linux")]
            {
                ("systemctl", &["suspend"])
            }
            #[cfg(target_os = "macos")]
            {
                ("pmset", &["sleepnow"])
            }
            #[cfg(windows)]
            {
                ("rundll32.exe", &["powrprof.dll,SetSuspendState", "0,1,0"])
            }
        };

        Command::new(cmd).args(args).status().await?;
        Ok(())
    }

    /// Does lock the system
    pub async fn lock() -> Result<()> {
        let (cmd, args): (&str, &[&str]) = {
            #[cfg(target_os = "linux")]
            {
                ("loginctl", &["lock-session"])
            }
            #[cfg(target_os = "macos")]
            {
                ("open", &["-a", "loginwindow"])
            }
            #[cfg(windows)]
            {
                ("rundll32.exe", &["user32.dll,LockWorkStation"])
            }
        };

        Command::new(cmd).args(args).status().await?;
        Ok(())
    }

    /// Does logout the system
    pub async fn logout() -> Result<()> {
        let (cmd, args): (&str, &[&str]) = {
            #[cfg(target_os = "linux")]
            {
                ("loginctl", &["terminate-session", "self"])
            }
            #[cfg(target_os = "macos")]
            {
                (
                    "osascript",
                    &["-e", "tell application \"System Events\" to log out"],
                )
            }
            #[cfg(windows)]
            {
                ("shutdown", &["/l"])
            }
        };

        Command::new(cmd).args(args).status().await?;
        Ok(())
    }
}
