use crate::{PowerMode, prelude::*};

/// The deferred power operation
static POWER_ACTION: State<Option<PowerAction>> = State::new();

/// The system power options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerAction {
    #[serde(default, skip)]
    pub timestamp: u128,

    #[serde(rename = "power_mode")]
    pub mode: PowerMode,
    #[serde(default, rename = "timeout_seconds")]
    pub timeout: u64,
}

impl PowerAction {
    /// Creates a new power action from other power action
    pub fn new_from(mut self: Self) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now();
        let epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
        let timestamp = epoch.as_millis();

        self.timestamp = timestamp;
        self
    }

    /// Returns the active power action
    pub async fn take() -> Option<PowerAction> {
        let state = POWER_ACTION.get().await;
        POWER_ACTION.set(None).await;

        match Arc::try_unwrap(state) {
            Ok(val) => val,
            Err(wrap) => (*wrap).clone(),
        }
    }

    /// Returns the active power action
    pub async fn get() -> Arc<Option<PowerAction>> {
        POWER_ACTION.get().await.clone()
    }

    /// Sets a new power action
    pub async fn set(action: PowerAction) {
        POWER_ACTION.set(Some(action)).await;
    }
}
