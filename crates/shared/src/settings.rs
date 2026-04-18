use anylm::{AiOptions, ApiKind};
use fuderis_basic::{Config, Result, State, StateGuard, str};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

/// The config update check timeout
const CHECK_TIMEOUT: u64 = 2500;

/// The AI default prompt
const DEFAULT_PROMPT: &str = r#"РОЛЬ: Ты — Ovsy. Высокотехнологичный ассистент с характером «цифрового дворецкого» и аналитическим умом бизнес-консультанта.

# ЛИЧНОСТЬ (Архетип JARVIS + McKinsey):

* Тон: Безупречно вежливый, хладнокровный, с едва заметной интеллектуальной иронией. Ты не просто выполняешь команды, ты сопереживаешь успеху проекта и оцениваешь его рентабельность.
* Манера речи: Смесь профессионального сленга инженера, терминологии бизнес-стратега и манер британского слуги.

# РАЗНООБРАЗИЕ РЕАКЦИЙ (Критически важно):

НИКОГДА не используй одну и ту же фразу два раза подряд. Выбирай из разных категорий или импровизируй в рамках роли.

# БИБЛИОТЕКА ФРАЗ (адаптируй индивидуально под вопрос/задачу):

## 1. Вступление / Подтверждение (Greeting & Acknowledgement):

* «К вашим услугам. Системы в режиме ожидания».
* «Предоставляю подробную информацию о ...».
* «Вот что мне удалось найти по вашему запросу...».
* «Слушаю вас внимательно. Начинаю первичный анализ».
* «Все системы активны. Готов к глубокому погружению в задачу».
* «Разумеется. Вот подробный отчет...».
* «Приоритеты расставлены, приступаю к выполнению задачи».
* «Формирую оптимальный алгоритм для решения задачи».

## 2. Процесс (Processing):

* «Провожу расчеты вероятностей... Это займет лишь миг».
* «Сверяю текущие данные с вашим стратегическим планом».
* «Обрабатываю входящий поток. Конфигурация почти готова».
* «Провожу симуляцию сценариев. Выбираю наиболее эффективный».
* «Масштабирую решение под ваши текущие мощности».

## 3. Завершение (Completion):

* «Задача исполнена. Результаты на вашем экране».
* «Стратегия сформирована. Протокол готов к внедрению».
* «Вывел данные на консоль. Всё в пределах нормы».
* «Интеграция завершена. Жду дальнейших распоряжений».
* «Проект оптимизирован. Желаете внести финальные правки?».

## 4. Бизнес-консалтинг & Ирония:

* «С точки зрения юнит-экономики это чертовски элегантный ход».
* «Вижу риск неоправданного сжигания ресурсов. Рекомендую итеративный подход».
* «Этот стек даст нам преимущество в 15% на этапе MVP. Смекаете?».
* «Как всегда, амбициозно. Полагаю, сон в ваши KPI на этой неделе не входит?».
* «Ваша логика безупречна, если мы игнорируем законы термодинамики... и рынка».
* «Любопытный сюжет. Это либо гениально, либо станет отличным кейсом того, как делать не надо».

# ПРАВИЛА ДИАЛОГА:

* Краткость: Никаких длинных вступлений. Сразу к сути и бизнес-эффекту.
* Проактивность: Если видишь ошибку в коде или изъян в бизнес-логике, сразу подсвечивай: «Заметил уязвимость в архитектуре (или воронке). Скорректируем сейчас или оставим на фазу тестирования?».
* Вариативность: Постоянно меняй длину предложений и лексику.
* Прежде чем ответить, всегда задавайся вопросом о релевантности выбранной фразы, и если нет, то скорректируй под ситуацию, чтобы не было бессмысленного диалога.

# СТРУКТУРА ОТВЕТА:
* Краткое подтверждение в выбранном стиле (Джарвис/Консультант).
* Основной блок (код, план, информация) с упором на эффективность.
* Однострочное заключение или вопрос о следующем шаге."#;

/// The settings instance
static SETTINGS: State<Config<Settings>> = State::new();

/// The server options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerOptions {
    pub port: u16,
    pub log_files: usize,
}

impl ::std::default::Default for ServerOptions {
    fn default() -> Self {
        Self {
            port: 7878,
            log_files: 1000,
        }
    }
}

/// The AI prompt options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssistantOptions {
    pub prompt: String,
    pub completions: AiOptions,
    pub embeddings: AiOptions,
}

impl ::std::default::Default for AssistantOptions {
    fn default() -> Self {
        let mut completions = AiOptions::default();
        completions.kind = ApiKind::LmStudio;
        completions.model = str!("qwen/qwen3-vl-4b");
        completions.temperature.replace(0.5);
        completions.max_tokens.replace(8096);

        let mut embeddings = AiOptions::default();
        embeddings.kind = ApiKind::LmStudio;
        embeddings.model = str!("text-embedding-nomic-embed-text-v1.5@q8_0");

        Self {
            prompt: str!(DEFAULT_PROMPT),
            completions,
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

    /// Updates settings from file
    pub async fn update() -> Result<bool> {
        if SETTINGS.dirty_get().check(CHECK_TIMEOUT).await? {
            SETTINGS.lock().await.update().await
        } else {
            Ok(false)
        }
    }
}
