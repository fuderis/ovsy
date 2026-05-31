use crate::{power::PowerMode, prelude::*};

/// The power action status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename = "lowercase")]
pub enum PowerStatus {
    Executed,
    Deferred {
        mode: PowerMode,
        target_time: chrono::DateTime<chrono::Utc>,
        remaining_secs: u64,
    },
    Canceled {
        mode: PowerMode,
    },
    ActiveTask {
        mode: PowerMode,
        target_time: chrono::DateTime<chrono::Utc>,
        remaining_secs: u64,
    },
    NoActiveTask,
}
