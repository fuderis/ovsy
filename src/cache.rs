use crate::prelude::*;
use fuzzy_cmp as fz;
use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncSeekExt, AsyncWriteExt},
};

/// The comparing coefficient
const COMP_COEF: f32 = 0.8;

/// The cached keywords
#[derive(Default, Debug, Clone)]
pub struct CachedKeys {
    pub keys: HashSet<String>,
    pub size: usize,
}

/// The query cache data
#[derive(Default, Debug, Clone)]
pub struct AgentCache {
    pub path: PathBuf,
    pub keys: Arc<State<CachedKeys>>,
}

impl AgentCache {
    /// Reads or writes default agent cache file
    pub async fn read_or_write(dir: impl Into<PathBuf>) -> Result<Self> {
        let dir = dir.into();
        let path = dir.join("Ovsy.cache");

        // read/write file:
        if !path.exists() {
            fs::write(&path, []).await?;
        }
        let buffer = fs::read_to_string(&path).await?;
        let size = buffer.len();

        // parse keywords:
        let keys = buffer
            .split_whitespace()
            .into_iter()
            .filter(|s| !s.is_empty())
            .map(Into::into)
            .collect();

        Ok(Self {
            path,
            keys: Arc::new(State::from(CachedKeys { keys, size })),
        })
    }

    /// Refresh keywords if file changed
    pub async fn refresh_if_changed(&self) -> Result<()> {
        let mut guard = self.keys.lock().await;
        let cached = &mut *guard;

        let len = if let Ok(meta) = tokio::fs::metadata(&self.path).await {
            meta.len() as usize
        } else {
            0
        };

        if len > cached.size {
            // reading only a new data:
            let mut file = File::open(&self.path).await?;
            file.seek(std::io::SeekFrom::Start(cached.size as u64))
                .await?;

            let mut buf = String::new();
            file.read_to_string(&mut buf).await?;

            // parsing keywords:
            let new_keys = buf
                .split_whitespace()
                .filter(|s| !s.is_empty())
                .map(|s| s.trim().to_lowercase())
                .collect::<Vec<_>>();

            // insert a new keywords:
            for key in new_keys {
                cached.keys.insert(key);
            }

            cached.size = len;
        }

        Ok(())
    }

    /// Writed a new keywords to cache file
    pub async fn write_keys(&self, keys: HashSet<String>) -> Result<()> {
        self.refresh_if_changed().await?;
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(&self.path)
            .await?;

        for key in keys {
            let key = key.trim().to_lowercase();

            if self.keys.lock().await.keys.insert(key.clone()) {
                file.write(key.as_bytes()).await?;
                file.write(b" ").await?;
            }
        }

        Ok(())
    }

    /// Returns comparing similarity (0.0 = 0%, 1.0 = 100%)
    pub async fn compare(&self, words: &[String]) -> Result<bool> {
        self.refresh_if_changed().await?;

        let mut score = 0.0;
        let count = words.len();
        let mut bonus = true;

        for word in words.iter() {
            if !fz::search(
                &self.keys.unsafe_get().keys.iter().collect::<Vec<_>>(),
                word,
                COMP_COEF,
                false,
            )
            .is_empty()
            {
                score += 1.0;
                if bonus {
                    score += 1.0;
                }
            } else {
                bonus = false
            }
        }

        Ok(score * 1.0 / count as f32 >= COMP_COEF)
    }

    /// Splits string to words
    pub fn to_words(s: &str) -> Vec<String> {
        s.trim()
            .to_lowercase()
            .split_whitespace()
            .into_iter()
            .map(Into::into)
            .collect()
    }
}
