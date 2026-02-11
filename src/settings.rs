use crate::prelude::*;

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

/*  TODO: AI context
///The AI context settings
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct ContextSettings;

impl ::std::default::Default for ContextSettings {
    fn default() -> Self {
        Self {}
    }
}
*/

/// The tools settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolsSettings {
    pub dirs: Vec<PathBuf>,
    pub autocheck: bool,
    pub check_timeout: u64,
    pub trace_timeout: u64,
    pub recurs_limit: usize,
    pub history_limit: usize,
}

impl ::std::default::Default for ToolsSettings {
    fn default() -> Self {
        Self {
            dirs: vec![path!("$/../../tools/pc-control")],
            autocheck: true,
            check_timeout: 2000,
            trace_timeout: 200,
            recurs_limit: 10,
            history_limit: 8192,
        }
    }
}

/// The LM Studio settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LMStudioAPISettings {
    pub port: u16,
    pub small: (String, u32, f32),
    pub large: (String, u32, f32),
}

impl ::std::default::Default for LMStudioAPISettings {
    fn default() -> Self {
        Self {
            port: 9090,
            small: (str!("qwen2.5-coder-3b-instruct"), 4096, 0.2),
            large: (str!("qwen/qwen3-vl-8b"), 8192, 0.4),
        }
    }
}

/// The LM API type
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum LMKind {
    #[serde(rename = "lm-studio")]
    LMStudio,
}

/// The LM's settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LMSSettings {
    pub slm_kind: LMKind,
    pub llm_kind: LMKind,
}

impl ::std::default::Default for LMSSettings {
    fn default() -> Self {
        Self {
            slm_kind: LMKind::LMStudio,
            llm_kind: LMKind::LMStudio,
        }
    }
}

/// The settings
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub server: ServerSettings,
    // pub context: ContextSettings,
    pub tools: ToolsSettings,
    pub lmstudio: LMStudioAPISettings,
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
