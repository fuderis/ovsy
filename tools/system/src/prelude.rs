#![allow(unused_imports)]
pub use crate::{Error, Result, Settings, StdResult, app_data, utils};
pub use axum::{
    Json, Router,
    extract::{Path as Paths, Query},
    http::StatusCode,
    response::Html,
    routing::{get, post},
};

// Tools:
pub use atoman::{Config, Flag, Lazy, Logger, State, StateGuard, error as err, info, lazy, warn};
pub use macron::{Display, From, hash_map as map, hash_set as set, path, re, str};

// Serde:
pub use serde::{Deserialize, Serialize, de::DeserializeOwned};
pub use serde_json::{self as json, Value as JsonValue, json};
// pub use toml::{ self, Value as TomlValue };

// STD:
pub use std::{
    fmt::Debug as Debugging,
    format as fmt,
    // time::std_sleep,
    // pin::Pin,
    // future::Future,
    net::SocketAddr,
    // collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex as StdMutex},
    time::Duration,
};

// Tokio:
pub use tokio::{
    net::TcpListener,
    sync::Mutex,
    time::{Instant, sleep},
};
