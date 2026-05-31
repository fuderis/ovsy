use super::VolumeMode;
use crate::prelude::*;

/// The volume action data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeAction {
    pub mode: VolumeMode,
    pub value: Option<i32>,
}
