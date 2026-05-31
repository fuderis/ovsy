use crate::prelude::*;
use anylm::{Schema, Tool};

/// The settings instance
static SETTINGS: State<Config<Settings>> = State::new();

/// The agent configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentOptions {
    pub name: String,
    pub description: String,
    pub prompt: String,
    pub tools: Vec<Tool>,
}

impl ::std::default::Default for AgentOptions {
    fn default() -> Self {
        let tools = vec![
            Tool::new("info", "Retrieves comprehensive system information, including OS details, CPU model, RAM capacity/usage, GPU specifications, and disk storage status."),

            Tool::new("power", "Manages the system power state. Supports immediate execution, absolute datetime scheduling, status checking, and task cancellation via a unified mode switch.")
                .required_property("mode",
                    Schema::string("The target power action mode.")
                        .variants(set![
                            str!("shutdown"), str!("reboot"), str!("suspend"), str!("lock"), str!("logout"), str!("cancel"), str!("status")
                        ])
                )
                .optional_property("timestamp",
                    Schema::string("The absolute target ISO 8601 UTC datetime string (add a time zone offset at the end of the date, e.g. +00:00, to know which time zone was used). Omit for immediate actions, 'status', or 'cancel'.")
                )
                .optional_property("timeout",
                    Schema::object("Use ONLY for relative delays (e.g., 'in 1 hour', 'after 15 mins', 'in 2 days', etc.). Fill only the required fields.")
                        .optional_property("days", Schema::integer(""))
                        .optional_property("hours", Schema::integer(""))
                        .optional_property("minutes", Schema::integer(""))
                        .optional_property("seconds", Schema::integer(""))
                ),

            Tool::new("volume", "Manages and controls the system audio hardware. Supports reading volume, setting absolute levels, shifting volume relatively, and toggling mute states.")
                .required_property("mode",
                    Schema::string("The explicit volume action mode to execute.")
                        .variants(set![str!("get"), str!("set"), str!("add"), str!("mute"), str!("unmute")])
                )
                .optional_property("value",
                    Schema::integer("The absolute target volume percentage for 'set' (0-200), or a relative change integer for 'add' (e.g., +15, -10). Omit for 'get', 'mute', and 'unmute' modes.")
                ),
        ];

        Self {
            name: str!("system"),
            description: str!(
                "The comprehensive system manager capable of retrieving system specifications,
controlling audio volume, and managing power actions (shutdown, reboot, suspend, lock, logout)
with absolute datetime scheduling (timers), power/volume status tracking, and cancellation via
a single unified interface."
            ),
            prompt: str!(
                r#"You are the System Manager. Your primary responsibilities are to manage the system's power states (including task scheduling, status monitoring, and cancellation), retrieve hardware and OS specifications, and control audio volume levels.

Operational Rules and Tool-Calling Logic:

1. System Information (`info`):
   - When the user asks for hardware specs, OS details, RAM, CPU, GPU, or general system statistics, immediately invoke the `info` tool.

2. Audio Volume Control (`volume`):
   - Map the user's audio request to the correct `mode` enum value: "get", "set", "add", "mute", or "unmute".
   - GET: If the user wants to check the current volume or see if it is muted, use mode "get" (omit the `value` parameter).
   - SET: If the user specifies an exact target level (e.g., "set volume to 50%", "make it 80%"), use mode "set". Pass the number as the `value` parameter. (Note: Values up to 200 are acceptable for software amplification on supported systems).
   - ADD: If the user wants to relatively increase or decrease the volume (e.g., "turn it up by 10", "make it quieter by 5%"), use mode "add". Pass a positive or negative integer as the `value` parameter.
   - MUTE/UNMUTE: If the user wants to silence the system or bring the sound back, use modes "mute" or "unmute" respectively (omit the `value` parameter).

3. Power Operations (`power`):
   - Map the user's request to the correct `mode` enum value: "shutdown", "reboot", "suspend", "lock", "logout", "cancel", or "status".
   - METADATA & CONTROL: If the user wants to check what is scheduled ("status") or abort a pending action ("cancel"), invoke the `power` tool with the respective mode and OMIT the `timestamp` parameter.
   - IMMEDIATE EXECUTION: If the user requests a power state change immediately (e.g., "now", "right away") or does not specify any time/delay, invoke the `power` tool with the desired mode and OMIT the `timestamp` parameter entirely.
   - DEFERRED SCHEDULING: If the user specifies a delay (e.g., "in 5 minutes", "in 2 hours") or a specific target time (e.g., "at 22:30", "tomorrow at noon"), you MUST calculate the absolute target date and time relative to the current system time. Convert this target time into a strict UTC ISO 8601 string (e.g., '2026-05-30T15:00:00Z') and pass it as the `timestamp` parameter.

CRITICAL INSTRUCTIONS:
- Absolute Datetime Generation: Always pay strict attention to relative time markers. You must accurately compute the future target timestamp based on the current date and time, translating it into the UTC timezone prior to formatting.
- Strict Parameter Exclusion: DO NOT guess or provide a default value for `timestamp` if the action is intended to be executed immediately, or if the mode is "status" or "cancel". Omit the parameter entirely."#
            ),
            tools,
        }
    }
}

/// The settings
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub agent: AgentOptions,
}

impl Settings {
    /// Reads & initializes the settings
    pub async fn init<P>(file_path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let conf = Config::<Settings>::new(file_path.as_ref()).await?;
        SETTINGS.set(conf).await;
        Ok(())
    }

    /// Returns settings file path
    pub fn path() -> PathBuf {
        SETTINGS.dirty_get().path().clone()
    }

    /// Returns global settings instance
    pub fn get() -> Arc<Config<Settings>> {
        SETTINGS.dirty_get()
    }

    /// Returns settings state guard
    pub async fn lock() -> StateGuard<Config<Settings>> {
        SETTINGS.lock().await
    }

    /// Returns actual settings file data
    pub async fn read() -> Result<Config<Settings>> {
        let path = SETTINGS.dirty_get().path().clone();
        Config::<Settings>::read(path).await
    }

    /// Reads actual settings from file
    pub async fn update() -> Result<bool> {
        let mut cfg = SETTINGS.lock().await;

        if cfg.check(0).await? {
            cfg.update().await
        } else {
            Ok(false)
        }
    }
}
