#![allow(unused_imports)]
pub use fudes_server::*;
pub use ovsy_shared::{app_data, Settings};

pub use serde::{Deserialize, Serialize};
pub use serde_json::json;
pub use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
pub use tokio::{sync::Mutex, time::Instant};

/// The stream sender alias
pub type Sender = Arc<StreamSender<Bytes>>;
