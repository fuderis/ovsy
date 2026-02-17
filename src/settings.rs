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

/// The tools settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolsSettings {
    pub dirs: Vec<PathBuf>,
    pub autocheck: bool,
    pub check_timeout: u64,
    pub trace_timeout: u64,
    pub recurs_limit: usize,
}

impl ::std::default::Default for ToolsSettings {
    fn default() -> Self {
        Self {
            dirs: vec![path!("$/../../tools/pc-control")],
            autocheck: true,
            check_timeout: 2000,
            trace_timeout: 200,
            recurs_limit: 10,
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
    pub tools: ToolsSettings,
    pub lms: LMSSettings,
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
