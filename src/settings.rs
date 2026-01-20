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

/// The tools settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolsSettings {
    pub dirs: Vec<PathBuf>,
    pub timeout: u64,
}

impl ::std::default::Default for ToolsSettings {
    fn default() -> Self {
        Self {
            dirs: vec![
                #[cfg(debug_assertions)]
                {
                    path!("$/../../tools")
                },
                #[cfg(not(debug_assertions))]
                {
                    path!("$/tools")
                },
            ],
            timeout: 2500,
        }
    }
}

/// The LM Studio settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LMStudioSettings {
    pub port: u16,
    pub exec: PathBuf,
}

impl ::std::default::Default for LMStudioSettings {
    fn default() -> Self {
        Self {
            port: 9090,
            exec: path!("/opt/lmstudio/LMStudio.AppImage"),
        }
    }
}

/// The LM-API type
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum LMKind {
    #[serde(rename = "lm-studio")]
    LMStudio,
}

/// The Small LM settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SLMSettings {
    pub kind: LMKind,
    pub token: String,
    pub model: String,
    pub context: u32,
}

impl ::std::default::Default for SLMSettings {
    fn default() -> Self {
        Self {
            kind: LMKind::LMStudio,
            token: str!(""),
            model: str!("qwen2.5-coder-3b-instruct"),
            context: 4096,
        }
    }
}

/// The Large LM settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LLMSettings {
    pub kind: LMKind,
    pub token: String,
    pub model: String,
    pub context: u32,
}

impl ::std::default::Default for LLMSettings {
    fn default() -> Self {
        Self {
            kind: LMKind::LMStudio,
            token: str!(""),
            model: str!("qwen/qwen3-vl-8b"),
            context: 8192,
        }
    }
}

/// The settings
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub server: ServerSettings,
    pub tools: ToolsSettings,
    pub lmstudio: LMStudioSettings,
    pub slm: SLMSettings,
    pub llm: LLMSettings,
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
