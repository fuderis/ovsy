use crate::prelude::*;

/// The settings instance
static SETTINGS: State<Config<Settings>> = State::new();

/// The server settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerSettings {
    pub port: u16,
}

impl ::std::default::Default for ServerSettings {
    fn default() -> Self {
        Self {
            port: 7879,
        }
    }
}

/// The music settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MusicSettings {
    pub dirs: Vec<PathBuf>,
}

impl ::std::default::Default for MusicSettings {
    fn default() -> Self {
        Self {
            dirs: vec![path!("~/Music")],
        }
    }
}

/// The settings
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub server: ServerSettings,
    pub music: MusicSettings,
}

impl Settings {
    /// Reads & initializes the settings
    pub fn init<P>(file_path: P) -> Result<()>
    where
        P: AsRef<Path>
    {
        let cfg = Config::new(file_path.as_ref())?;
        SETTINGS.unsafe_set(cfg);
        Ok(())
    }
    
    /// Returns settings instance state
    pub fn get() -> Arc<Config<Settings>> {
        SETTINGS.unsafe_get()
    }

    /// Returns settings instance mutex guard
    pub async fn lock() -> StateGuard<Config<Settings>> {
        SETTINGS.lock().await
    }

    /// Updates settings from file
    pub async fn update() -> Result<()> {
        SETTINGS.lock().await.update()
    }
}
