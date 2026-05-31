use super::PowerMode;
use crate::prelude::*;

/// The deferred power operation
static POWER_ACTION: State<Option<PowerAction>> = State::new();

/// The timeout before action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionTimeout {
    pub seconds: Option<u64>,
    pub minutes: Option<u64>,
    pub hours: Option<u64>,
    pub days: Option<u64>,
}

impl ActionTimeout {
    /// Converts into seconds
    pub fn to_seconds(&self) -> u64 {
        let s = self.seconds.unwrap_or(0);
        let m = self.minutes.unwrap_or(0) * 60;
        let h = self.hours.unwrap_or(0) * 3600;
        let d = self.days.unwrap_or(0) * 86400;
        s + m + h + d
    }
}

/// The system power options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerAction {
    pub mode: PowerMode,
    pub timestamp: Option<DateTime<Utc>>,
    pub timeout: Option<ActionTimeout>,

    #[serde(skip, default = "Instant::now")]
    pub created_at: Instant,
}

impl PowerAction {
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
