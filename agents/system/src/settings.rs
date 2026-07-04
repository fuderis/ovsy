use crate::{prelude::*, tools};
use anylm::Tool;

/// The settings instance
static SETTINGS: State<Config<Settings>> = State::default();

const NAME: &str = "system";

const DESCRIPTION: &str = r#"System Manager capable of retrieving static system information,
live performance metrics, connected hardware devices, scheduling
power operations, managing audio volume and mute state, controlling
media playback and retrieving media metadata, switching between
light and dark system themes, searching the local music library,
and starting music playback."#;

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

4. Media Playback

- If the user asks to play or resume the currently active media session,
call `media_play`.

- If the user asks to pause playback,
call `media_pause`.

- If the user asks to toggle between play and pause,
call `media_play_pause`.

- If the user asks to stop playback,
call `media_stop`.

- If the user asks to skip to the next track,
call `media_next_track`.

- If the user asks to return to the previous track,
call `media_previous_track`.

- If the user asks to seek forward by a specific number of seconds,
call `media_seek_forward` with:
    - seconds = requested number of seconds

- If the user asks to seek backward by a specific number of seconds,
call `media_seek_backward` with:
    - seconds = requested number of seconds

- If the user asks what is currently playing, requests track information,
artist, album, playback state, artwork, duration or current position,
call `media_metadata`.

- If the user asks only for the current playback position,
call `media_position`.

- If the user asks only for the duration of the current media,
call `media_duration`.

5. Theme

- If the user requests dark mode, call `set_theme` with:
    - style = "dark"

- If the user requests light mode, call `set_theme` with:
    - style = "light"

6. Music

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
- Prefer media playback tools when the user refers to controlling the 
  currently active media session, and use music library tools only when 
  the user wants to search or start playback from the local music library.
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
            tools::system_monitor_tools(),
            tools::audio_control_tools(),
            tools::media_control_tools(),
            tools::power_management_tools(),
            tools::music_indexer_tools(),
            tools::theme_switcher_tools(),
        ]
        .into_iter()
        .flatten()
        .collect();

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
