use crate::prelude::*;

/// The IR-code action
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Action {
    // Media:
    PlayPause,
    Stop,
    VolumeUp,
    VolumeDown,
    Mute,
    NextTrack,
    PrevTrack,
    ScrollLeft,
    ScrollRight,

    // Workspace:
    TabNext,
    TabPrev,
    WinTabNext,
    WinTabPrev,
    WinNextSpace,
    WinPrevSpace,

    // Power:
    PowerOff,
    Sleep,
}
