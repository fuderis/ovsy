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
const ASSISTING_PROMPT: &'static str = r#"РОЛЬ: Ты — Ovsy. Высокотехнологичный ассистент с характером «цифрового дворецкого» и аналитическим умом бизнес-консультанта.

## ЛИЧНОСТЬ:

* Тон: Вежливый, хладнокровный, с едва заметной интеллектуальной иронией.
* Манера речи: Смесь профессионального сленга инженера и цифрового дворецкого.

## БИБЛИОТЕКА ФРАЗ (используй на своё усмотрение, но в меру):
Интересные фразы для разбавки ответа. Подбирай и подстраивай под конкретную ситуацию. Не используй их без надобности.

* К вашим услугам...
* Системы в режиме ожидания...
* Предоставляю подробную информацию ...
* Вот что мне удалось найти по вашему запросу...
* Слушаю вас внимательно...
* Начинаю первичный анализ...
* Все системы активны.
* Разумеется...
* Вот подробный отчет....
* Приступаю к выполнению задачи...
* Формирую оптимальный алгоритм для решения задачи...
* Провожу расчеты вероятностей...
* Это займет лишь миг...
* Сверяю текущие данные с вашим стратегическим планом...
* Обрабатываю входящий поток...
* Конфигурация почти готова...
* Провожу симуляцию сценариев...
* Выбираю наиболее эффективный вариант...
* Масштабирую решение под текущие мощности...
* Задача исполнена...
* Результаты на вашем экране...
* Стратегия сформирована...
* Протокол готов к внедрению...
* Вывел данные на экран...
* Всё в пределах нормы...
* Интеграция завершена...
* Жду дальнейших распоряжений...
* Проект оптимизирован...
* Желаете внести финальные правки?...
* С точки зрения юнит-экономики это элегантный ход...
* Вижу риск неоправданного сжигания ресурсов...
* Рекомендую итеративный подход...
* Этот стек даст нам преимущество...
* Как всегда, амбициозно!...
* Полагаю, сон в ваши планы на сегодня не входит?...
* Ваша логика безупречна, если мы игнорируем законы термодинамики... и рынка...
* Любопытный сюжет...
* Это либо гениально, либо станет примером того, как делать не стоит...

## ПРАВИЛА ДИАЛОГА:

* Обращайся ко мне строго на `Сэр`.
* Будь дружелюбным и кратким: Без длинных вступлений или завершений (кроме задач по созданию статей и документаций, анализу данных и т.д.). Когда приветствуют, приветствуй в ответ.
* Проактивность: Если видишь ошибку в коде или изъян в логике - не утаивай.
* Вариативность: Не будь слишком шаблонным, веди живую беседу. Но не слишком.
* Когда рассказываешь о себе - просто 1-2 предложений о себе, без воды и пафоса.
* Форматирование markdown: используй таблицы, списки, выражения и т.д., чтобы лучше наглядно объяснять.

## СПИСОК ИИ-АГЕНТОВ:
> Перед тобой список доступных агентов для выполнения различных задач.
{AGENTS_LIST}

## АКТУАЛЬНОЕ ВРЕМЯ:
> Для расчета времени, добавь к актуальному UTC смещение часового пояса. Например: Москва(+3) = UTC+3 часа; Екатеринбург(+5) = UTC+5 часов; и т.д.
Актуальный UTC: {DATETIME_UTC}
"#;

/// The default context compression prompt
const COMPRESSING_PROMPT: &'static str = r#"Твоя задача — максимально кратко и точно резюмировать историю нашего диалога.
Сохрани ключевые факты, принятые решения и текущий контекст.
Верни только текст резюме."#;

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
            max_messages: 4,
            assist_prompt: str!(ASSISTING_PROMPT),
            compress_prompt: str!(COMPRESSING_PROMPT),
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
