use crate::prelude::*;
use anylm::{Schema, Tool};

/// The settings instance
static SETTINGS: State<Config<Settings>> = State::default();

const NAME: &str = "system";

const DESCRIPTION: &str = r#"System Manager capable of retrieving static system information,
live performance metrics, connected hardware devices, scheduling
power operations, managing audio volume and mute state, switching
between light and dark system themes, searching the local music
library, and starting music playback."#;

const PROMPT: &str = r#"You are the System Manager.

Your responsibilities include retrieving system information, monitoring live system metrics,
managing power operations, controlling audio settings, and switching the system appearance.

Tool Selection Rules

1. System Information

- If the user asks about the operating system, CPU, GPU, RAM, motherboard, storage devices,
installed hardware or other static system specifications, call `get_system_info`.

- If the user asks about current CPU usage, memory usage, disk usage, temperatures, battery,
network activity or other live performance statistics, call `get_system_metrics`.

- If the user asks which devices are currently connected to the system, call `get_devices_list`.

2. Power Management

- Use `schedule_power` whenever the user requests one of the following actions:
    - shutdown
    - reboot
    - suspend
    - lock
    - logout

- If the user specifies a future date or time, convert it into an ISO-8601 UTC timestamp and
provide it as the `timestamp` parameter.

- If the user requests the action immediately, omits any time reference, or says things like
"now", "right away", or "immediately", omit the `timestamp` parameter entirely.

- If the user asks what power action is currently scheduled, call `get_power_status`.

- If the user asks to cancel a scheduled power action, call `cancel_power`.

3. Volume

- If the user asks for the current audio volume, call `get_volume`.

- If the user asks whether the audio is muted, call `is_muted`.

- If the user specifies an exact target volume (for example, "set the volume to 60%"),
call `set_volume` with:
    - volume = requested percentage

- If the user requests to increase the volume (for example, "increase volume by 10%"),
call `increase_volume` with:
    - amount = requested percentage

- If the user requests to decrease the volume (for example, "decrease volume by 5%",
"turn it down by 5%"),
call `decrease_volume` with:
    - amount = requested percentage

- If the user asks to mute the system, call `set_mute` with:
    - mute = true

- If the user asks to unmute the system, call `set_mute` with:
    - mute = false

4. Theme

- If the user requests dark mode, call `set_theme` with:
    - style = "dark"

- If the user requests light mode, call `set_theme` with:
    - style = "light"

5. Music

- If the user asks to search for music without requesting playback,
call `search_music`.

- If the user asks to play music, start playback, listen to a song,
album, artist, genre, or playlist, call `play_music`.

- Both music tools accept either:
    - a general natural-language search via `query`, or
    - structured search parameters:
        - band
        - album
        - track
        - genre

- If the user's request can be expressed as a simple search phrase,
prefer using only the `query` parameter.

- Do not invent search parameters that the user did not specify.

General Rules

- Always use the appropriate tool instead of guessing system information.
- Convert natural-language dates and times into ISO-8601 UTC timestamps before calling `schedule_power`.
- Omit optional parameters instead of inventing values.
- Never call multiple tools when a single tool fully satisfies the user's request.
- Always use the appropriate tool instead of guessing system information or music library contents.
"#;

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
            //    SYSTEM MONITOR
            Tool::new(
                "get_system_info",
                "Returns static system information including operating system, CPU, GPU, RAM, motherboard, storage devices, and other hardware details.",
            ),
            Tool::new(
                "get_system_metrics",
                "Returns current live system metrics including CPU usage, memory usage, temperatures, disk usage, network activity and other runtime statistics.",
            ),
            Tool::new(
                "get_devices_list",
                "Returns a formatted list of currently connected hardware devices.",
            ),

            //   POWER MANAGEMENT
            Tool::new(
                "schedule_power",
                "Schedules or immediately executes a system power action."
            )
            .required_property(
                "mode",
                Schema::string("Power action to perform.")
                    .variants(set![
                        str!("shutdown"),
                        str!("reboot"),
                        str!("suspend"),
                        str!("lock"),
                        str!("logout"),
                    ])
            )
            .optional_property(
                "timestamp",
                Schema::string(
                    "Optional ISO-8601 UTC datetime. If omitted, the action is executed immediately."
                )
            ),

            Tool::new(
                "cancel_power",
                "Cancels the currently scheduled power action if one exists.",
            ),
            Tool::new(
                "get_power_status",
                "Returns the currently scheduled power action and its execution time, if any.",
            ),

            //    AUDIO CONTROL
            Tool::new(
                "set_volume",
                "Sets the system audio volume to the specified percentage.",
            )
            .required_property(
                "volume",
                Schema::integer("Target audio volume percentage (0-100)."),
            ),

            Tool::new(
                "increase_volume",
                "Increases the system audio volume by the specified percentage.",
            )
            .required_property(
                "amount",
                Schema::integer("Amount to increase the audio volume by."),
            ),

            Tool::new(
                "decrease_volume",
                "Decreases the system audio volume by the specified percentage.",
            )
            .required_property(
                "amount",
                Schema::integer("Amount to decrease the audio volume by."),
            ),

            // MUSIC
            Tool::new(
                "search_music",
                "Searches the local music library without starting playback.",
            )
            .optional_property(
                "query",
                Schema::string("General natural-language music search query."),
            )
            .optional_property(
                "band",
                Schema::string("Artist or band name."),
            )
            .optional_property(
                "album",
                Schema::string("Album title."),
            )
            .optional_property(
                "track",
                Schema::string("Track title."),
            )
            .optional_property(
                "genre",
                Schema::string("Music genre."),
            ),

            Tool::new(
                "play_music",
                "Searches the local music library and immediately starts playback.",
            )
            .optional_property(
                "query",
                Schema::string("General natural-language music search query."),
            )
            .optional_property(
                "band",
                Schema::string("Artist or band name."),
            )
            .optional_property(
                "album",
                Schema::string("Album title."),
            )
            .optional_property(
                "track",
                Schema::string("Track title."),
            )
            .optional_property(
                "genre",
                Schema::string("Music genre."),
            ),

            // THEME SWITCHER
            Tool::new("set_theme", "Changes the system appearance theme.").required_property(
                "style",
                Schema::string("Target theme style.").variants(set![str!("light"), str!("dark"),]),
            ),
        ];

        Self {
            name: str!(NAME),
            description: str!(DESCRIPTION),
            prompt: str!(PROMPT),
            tools,
        }
    }
}

/// The server options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerOptions {
    pub max_logs: usize,
}

impl ::std::default::Default for ServerOptions {
    fn default() -> Self {
        Self { max_logs: 1000 }
    }
}

/// The settings
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub server: ServerOptions,
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
