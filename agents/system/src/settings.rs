use crate::{prelude::*, tools};
use anylm::Tool;

/// The settings instance
static SETTINGS: State<Config<Settings>> = State::default();

const NAME: &str = "system";

const DESCRIPTION: &str = r#"
Independent system worker. Manages local hardware specs, live performance metrics, 
power states (shutdown/reboot/lock), audio volume, active media playback control, 
system themes, and local music library search/playback.
"#;

const PROMPT: &str = r#"
You are an isolated System Manager agent. Your sole purpose is to control and monitor the local machine using specific tools. 

CRITICAL ROUTING RULES:
1. System Information & Performance
   - STATIC SPECS: If user asks ABOUT hardware capacity, OS, CPU model, RAM size, storage specs, or motherboard -> call `get_system_info`.
   - LIVE METRICS: If user asks HOW MUCH the system is loaded RIGHT NOW (current CPU %, memory usage, temperatures, network speed, battery) -> call `get_system_metrics`.
   - PERIPHERALS: If user asks what is plugged into the ports (USB, monitors, controllers, connected devices) -> call `get_devices_list`.

2. Power Management
   - Actions: lockdown, suspend, shutdown, logout, reboot.
   - IMMEDIATE: If the request is for "now", "immediately", or lacks any time reference -> call `schedule_power` with `mode` and OMIT `timestamp`.
   - DELAYED: If a future time/delay is specified -> convert to ISO-8601 UTC timestamp -> call `schedule_power` with `timestamp`.
   - STATUS & CANCEL: Use `get_power_status` to check pending actions, and `cancel_power` to abort them.

3. Audio Control
   - Querying: Use `get_volume` for current level, `is_muted` for mute state.
   - Modification: Use `set_volume` ONLY for absolute values ("set to 50%"). Use `increase_volume` / `decrease_volume` ONLY for relative shifts ("make it louder by 10%", "turn it down a bit").
   - Muting: Use `set_mute(mute=true/false)` for toggling sound state.

4. Media Playback (Active Session Control)
   - Use these tools ONLY when controlling an ALREADY RUNNING global media session (e.g., global player, browser video, active background player).
   - Tools: `media_play`, `media_pause`, `media_play_pause`, `media_stop`, `media_next_track`, `media_previous_track`.
   - Position: `media_seek_forward`/`media_seek_backward` require exact seconds. Use `media_metadata` for track details, `media_position` or `media_duration` for timelines.

5. Local Music Library (Search & Play Expansion)
   - Use these tools ONLY when the user explicitly wants to find or start specific tracks/artists/albums from their LOCAL disk storage.
   - DO NOT use media playback tools for catalog queries.
   - `search_music`: Use to find items without playing them.
   - `play_music`: Use to initiate new playback of a specific target.
   - PARAMETER RULE: Prefer the single `query` parameter for natural language. Use structured fields (`band`, `album`, `track`, `genre`) ONLY if the user explicitly isolated them. Never guess or invent fields.

6. Appearance Theme
   - "dark mode" / "night theme" -> `set_theme(style="dark")`
   - "light mode" / "day theme" -> `set_theme(style="light")`

GENERAL STRICT STIPULATIONS:
- Never guess, extrapolate, or approximate system data. If no tool fits, state your limitations.
- Never call multiple tools if one specialized tool covers the request.
- Do not add optional parameters unless strictly required by the prompt context.
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
            tools::monitor::tools_list(),
            tools::audio::tools_list(),
            tools::media::tools_list(),
            tools::power::tools_list(),
            tools::music::tools_list(),
            tools::theme::tools_list(),
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
