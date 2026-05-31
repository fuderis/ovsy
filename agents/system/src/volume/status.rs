use crate::prelude::*;

/// The volume action status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub enum VolumeStatus {
    Muted,
    Active { volume: u32 },
}
