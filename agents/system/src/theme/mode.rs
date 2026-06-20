use crate::prelude::*;

/// The system theme mode
#[derive(Debug, Display, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "lowercase")]
#[display(rename = "lowercase")]
pub enum ThemeMode {
    Light,
    Dark,
}
