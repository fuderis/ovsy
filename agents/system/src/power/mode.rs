use crate::prelude::*;

/// The power mode
#[derive(Debug, Clone, Copy, Display, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[display(rename = "lowercase")]
pub enum PowerMode {
    Shutdown,
    Suspend,
    Reboot,
    Lock,
    Logout,
    Cancel,
    Status,
}
