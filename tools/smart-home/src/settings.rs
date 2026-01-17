use crate::prelude::*;

/// The settings instance
static SETTINGS: State<Config<Settings>> = State::new();

/// The music settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArduinoSettings {
    pub port: u16,
    pub rate: u32,
}

impl ::std::default::Default for ArduinoSettings {
    fn default() -> Self {
        Self {
            port: 3,
            rate: 115200,
        }
    }
}

/// The settings
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub arduino: ArduinoSettings,
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
    
    /// Updated settings & returns it
    pub fn get_updated() -> Result<Arc<Config<Settings>>> {
        Self::update()?;
        Ok(Self::get())
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
