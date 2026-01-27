#![allow(unused_imports)]
pub use crate::{Error, Result, Settings, StdResult, Tools, app_data};
pub use axum::{
    Json, Router,
    extract::{Path as Paths, Query},
    http::StatusCode,
    response::Html,
    routing::{get, post},
};

// Tools:
pub use atoman::{
    Config, Flag, Lazy, Logger, State, StateGuard, Trace, error as err, info, lazy, trace, warn,
};
pub use macron::{Display, From, hash_map as map, hash_set as set, path, re, str};
pub use uuid::Uuid;

// Chrono:
pub use chrono::{DateTime, Local, Utc};

// Serde:
pub use serde::{Deserialize, Serialize, de::DeserializeOwned};
pub use serde_json::{self as json, Value as JsonValue, json};
// pub use toml::{ self, Value as TomlValue };

// STD:
pub use std::{
    collections::HashMap,
    fmt::Debug as Debugging,
    format as fmt,
    // time::std_sleep,
    // pin::Pin,
    // future::Future,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex as StdMutex},
    time::{Duration, SystemTime},
};

// Tokio:
pub use tokio::{net::TcpListener, sync::Mutex, time::sleep};
