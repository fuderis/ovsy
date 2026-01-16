use crate::prelude::*;

/// The settings instance
static SETTINGS: State<Config<Settings>> = State::new();

/// The music settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MusicSettings {
    pub dirs: Vec<PathBuf>,
}

impl ::std::default::Default for MusicSettings {
    fn default() -> Self {
        Self {
            dirs: vec![path!("D:/Music")],
        }
    }
}

/// The settings
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub music: MusicSettings,
}

impl Settings {
    /// Reads & initializes the settings
    pub fn init<P>(file_path: P) -> Result<()>
    where
        P: AsRef<Path>
    {
        let cfg = Config::new(file_path.as_ref())?;
        SETTINGS.set(cfg);
        Ok(())
    }
    
    /// Returns settings instance state
    pub fn get() -> Arc<Config<Settings>> {
        SETTINGS.get()
    }

    /// Returns settings instance mutex guard
    pub fn lock() -> StateGuard<'static, Config<Settings>> {
        SETTINGS.lock()
    }

    /// Updates settings from file
    pub fn update() -> Result<()> {
        SETTINGS.lock().update()
    }
}
