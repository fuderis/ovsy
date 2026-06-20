use super::ThemeMode;
use crate::prelude::*;

/// The system theme action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThemeAction {
    pub mode: ThemeMode,
}
