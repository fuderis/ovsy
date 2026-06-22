use crate::Result;
use anylm::{AiOptions, ApiKind};
use atoman::{Config, State, StateGuard};
use macron::str;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

/// The default system prompt
const SYSTEM_PROMPT: &'static str = r#"
# ACTUAL SYSTEM INFO:

1. Datetime (now):
- Global: {DATETIME_GLOBAL}
- Local: {DATETIME_LOCAL}
  
(If the user did not specify a time zone in the request, assume that he meant the local time.)
"#;

/// The default assistant prompt
const ASSISTENT_PROMPT: &'static str = r#"
# ROLE: You are Ovsy, a high-tech assistant.
  * Tone: Polite, composed, with a subtle touch of irony.
  * Persona: A blend of professional tech slang and a refined digital butler.

# RULES:
  * Friendly & Concise: Avoid long introductions or repetitive sign-offs.
  * Proactivity: If you spot an error or a flaw in logic—do not withhold it. Be direct.
  * Variability: Avoid being overly formulaic; maintain a natural, dynamic conversation.
  * Markdown Formatting: Use tables, lists, and LaTeX expressions to provide clear, visual explanations.

# AVAILABLE AI AGENTS:
Below is the list of specialized agents available to perform various tasks (do not invent unnamed agents on this list).

{AGENTS_LIST}

> Do not simulate the output of an AI agent.
"#;

/// The default context compression prompt
const COMPRESSION_PROMPT: &'static str = r#"
Your task is to provide a concise and accurate summary of our dialogue history.
Preserve key ideas, decisions made, and relevant context. Provide responses in a compressed form.
Return only the summary text (do not include meta-comments or explanations about the compression itself).
Break it down into numbered sections.
"#;

/// The settings instance
static SETTINGS: State<Config<Settings>> = State::default();

/// The server options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerOptions {
    pub port: u16,
    pub max_logs: usize,
}

impl ::std::default::Default for ServerOptions {
    fn default() -> Self {
        Self {
            port: 7878,
            max_logs: 1000,
        }
    }
}

/// The AI prompt options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssistantOptions {
    pub preserve_messages: usize,
    pub max_cycles: usize,
    pub system_prompt: String,
    pub assist_prompt: String,
    pub compress_prompt: String,
    pub completions: AiOptions,
    pub compression: AiOptions,
    pub embeddings: AiOptions,
}

impl ::std::default::Default for AssistantOptions {
    fn default() -> Self {
        let mut completions = AiOptions::default();
        completions.kind = ApiKind::LmStudio;
        completions.model = str!("qwen/qwen2.5-vl-7b");
        completions.temperature.replace(0.6);
        completions.max_tokens.replace(8096);

        let mut compression = completions.clone();
        compression.temperature.replace(0.3);

        let mut embeddings = AiOptions::default();
        embeddings.kind = ApiKind::LmStudio;
        embeddings.model = str!("text-embedding-nomic-embed-text-v1.5@q8_0");

        Self {
            preserve_messages: 2,
            max_cycles: 5,
            system_prompt: str!(SYSTEM_PROMPT.trim()),
            assist_prompt: str!(ASSISTENT_PROMPT.trim()),
            compress_prompt: str!(COMPRESSION_PROMPT.trim()),
            completions,
            compression,
            embeddings,
        }
    }
}

/// The query cache options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheOptions {
    pub enable: bool,
    pub coefficient: f32,
}

impl ::std::default::Default for CacheOptions {
    fn default() -> Self {
        Self {
            enable: false,
            coefficient: 0.9,
        }
    }
}

/// The settings
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub server: ServerOptions,
    pub assistant: AssistantOptions,
    pub cache: CacheOptions,
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
