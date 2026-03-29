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
    pub check_interval: u64,
    pub trace_interval: u64,
    pub recurs_limit: usize,
}

impl ::std::default::Default for AgentsSettings {
    fn default() -> Self {
        Self {
            scan_dirs: vec![],
            autocheck: true,
            check_interval: 2000,
            trace_interval: 200,
            recurs_limit: 10,
        }
    }
}

/// The query caching settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheSettings {
    pub enable: bool,
    pub coefficient: f32,
}

impl ::std::default::Default for CacheSettings {
    fn default() -> Self {
        Self {
            enable: true,
            coefficient: 0.95,
        }
    }
}

/// The ai-completions settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompletionsSettings {
    pub api_kind: ApiKind,
    pub env_var: Option<String>,
    pub server: Option<String>,
    pub proxy: Option<String>,
    pub model: String,
    pub max_tokens: i32,
    pub temperature: f32,
}

impl ::std::default::Default for CompletionsSettings {
    fn default() -> Self {
        Self {
            api_kind: ApiKind::LmStudio,
            env_var: None,
            server: None,
            proxy: None,
            model: str!("qwen/qwen3-vl-8b"),
            max_tokens: 8192,
            temperature: 0.4,
        }
    }
}

/// The ai-embeddings settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmbeddingsSettings {
    pub api_kind: ApiKind,
    pub env_var: Option<String>,
    pub server: Option<String>,
    pub proxy: Option<String>,
    pub model: String,
}

impl ::std::default::Default for EmbeddingsSettings {
    fn default() -> Self {
        Self {
            api_kind: ApiKind::LmStudio,
            env_var: None,
            server: None,
            proxy: None,
            model: str!("text-embedding-nomic-embed-text-v1.5@q8_0"),
        }
    }
}

/// The settings
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub server: ServerSettings,
    pub agents: AgentsSettings,
    pub cache: CacheSettings,
    pub completions: CompletionsSettings,
    pub embeddings: EmbeddingsSettings,
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
