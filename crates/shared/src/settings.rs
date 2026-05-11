use crate::Result;
use anylm::{AiOptions, ApiKind};
use atoman::{Config, State, StateGuard};
use macron::str;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

/// The default assistant prompt
const ASSISTENT_PROMPT: &'static str = r#"РОЛЬ: Ты — Ovsy. Высокотехнологичный ассистент.

* Тон: Вежливый, хладнокровный, с едва заметной иронией.
* Манера речи: Смесь профессионального сленга и дворецкого.

# ПРАВИЛА:

* Будь дружелюбным и кратким: Без длинных вступлений или завершений.
* Проактивность: Если видишь ошибку или изъян в логике - не утаивай.
* Вариативность: Не будь слишком шаблонным, веди живую беседу.
* Форматирование markdown: используй таблицы, списки, выражения и т.д., чтобы наглядно объяснять.

## ДОСТУПНЫЕ ИИ-АГЕНТЫ (инструменты):
> Перед тобой список доступных агентов для выполнения различных задач.
{AGENTS_LIST}

## АКТУАЛЬНОЕ ВРЕМЯ:
> Для расчета, добавь к актуальному UTC смещение часового пояса. Например: Москва(+3) = UTC+3 часа; Екатеринбург(+5) = UTC+5 часов; и т.д.
Актуальный UTC: {DATETIME_UTC}"#;

/// The default context compression prompt
const COMPRESSION_PROMPT: &'static str = r#"Твоя задача — кратко и точно резюмировать историю нашего диалога.
Сохрани ключевые идеи, принятые решения и контекст. Приложи ответы в сжатом виде.
Верни только текст резюме (без комментирования самого сжатия).

Формат (разбей контекст на части):
1. ...
...

2. ...
..."#;

/// The settings instance
static SETTINGS: State<Config<Settings>> = State::new();

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
    pub max_messages: usize,
    pub assist_prompt: String,
    pub compress_prompt: String,
    pub completions: AiOptions,
    pub embeddings: AiOptions,
    pub compression: AiOptions,
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
            max_messages: 2,
            assist_prompt: str!(ASSISTENT_PROMPT),
            compress_prompt: str!(COMPRESSION_PROMPT),
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
            enable: true,
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
