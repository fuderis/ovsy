use crate::prelude::*;

/// The deferred power operation
pub static DEFERRED_POWER_ACTION: State<Option<PowerMode>> = State::new();

/// The power mode
#[derive(Clone, Copy, Display, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PowerMode {
    #[display = "turnoff"]
    TurnOff,
    #[display = "suspend"]
    Suspend,
    #[display = "reboot"]
    Reboot,
    #[display = "lock"]
    Lock,
}
