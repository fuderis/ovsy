use crate::prelude::*;

/// The settings instance
static SETTINGS: State<Config<Settings>> = State::new();

/// The music settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MusicSettings {
    pub scan_dirs: Vec<PathBuf>,
    pub search_coef: f32,
}

impl ::std::default::Default for MusicSettings {
    fn default() -> Self {
        Self {
            scan_dirs: vec![path!("~/Music")],
            search_coef: 0.65,
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
        P: AsRef<Path>,
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
