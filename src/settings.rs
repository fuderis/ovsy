use crate::prelude::*;
use anylm::ApiKind;

/// The settings instance
static SETTINGS: State<Config<Settings>> = State::new();

/// The Ovsy server settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerSettings {
    pub port: u16,
}

impl ::std::default::Default for ServerSettings {
    fn default() -> Self {
        Self { port: 7878 }
    }
}

/// The agents settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentsSettings {
    pub scan_dirs: Vec<PathBuf>,
    pub autocheck: bool,
    pub check_timeout: u64,
    pub trace_timeout: u64,
    pub recurs_limit: usize,
    pub caching: bool,
}

impl ::std::default::Default for AgentsSettings {
    fn default() -> Self {
        Self {
            scan_dirs: vec![],
            autocheck: true,
            check_timeout: 2000,
            trace_timeout: 200,
            recurs_limit: 10,
            caching: true,
        }
    }
}

/// The LM's settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LMSSettings {
    pub api_kind: ApiKind,
    pub env_var: String,
    pub server: String,
    pub proxy: String,
    pub model: String,
    pub max_tokens: i32,
    pub temperature: f32,
}

impl ::std::default::Default for LMSSettings {
    fn default() -> Self {
        Self {
            api_kind: ApiKind::LmStudio,
            env_var: str!(),
            server: str!(),
            proxy: str!(),
            model: str!("qwen/qwen3-vl-8b"),
            max_tokens: 8192,
            temperature: 0.4,
        }
    }
}

/// The settings
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub server: ServerSettings,
    pub agents: AgentsSettings,
    pub lms: LMSSettings,
}

impl Settings {
    /// Reads & initializes the settings
    pub fn init<P>(file_path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let mut cfg = Config::<Settings>::new(file_path.as_ref())?;

        #[cfg(debug_assertions)]
        {
            cfg.agents.scan_dirs.push(path!("$/../../agents"));
        }
        #[cfg(not(debug_assertions))]
        {
            cfg.agents.scan_dirs.push(path!("$/agents"));
        };

        SETTINGS.unsafe_set(cfg);
        Ok(())
    }

    /// Returns global settings instance
    pub fn get() -> Arc<Config<Settings>> {
        SETTINGS.unsafe_get()
    }

    /// Returns settings state guard
    pub async fn lock() -> StateGuard<Config<Settings>> {
        SETTINGS.lock().await
    }

    /// Returns actual settings file data
    pub fn read() -> Result<Config<Settings>> {
        let path = SETTINGS.unsafe_get().get_path().clone();
        Config::<Settings>::read(path)
    }
}
