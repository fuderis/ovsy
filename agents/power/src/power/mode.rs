use crate::prelude::*;

/// The power mode
#[derive(Debug, Clone, Copy, Display, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PowerMode {
    #[display = "shutdown"]
    Shutdown,
    #[display = "suspend"]
    Suspend,
    #[display = "reboot"]
    Reboot,
    #[display = "lock"]
    Lock,
}
