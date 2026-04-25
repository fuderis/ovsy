#![allow(unused_imports)]
pub use fudes_server::*;
pub use ovsy_shared::{Settings, app_data};

pub use serde::{Deserialize, Serialize};
pub use serde_json::json;
pub use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
pub use tokio::{sync::Mutex, time::Instant};

/// The stream sender alias
pub type Sender = Arc<StreamSender<Bytes>>;
