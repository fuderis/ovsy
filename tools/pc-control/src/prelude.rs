#![allow(unused_imports)]
pub use crate::{ Settings, StdResult, Result, Error, app_data };
pub use axum::{ routing::{ get, post }, http::StatusCode, response::Html, extract::{ Path as Paths, Query }, Json, Router };

// Tools:
pub use macron::{ Display, From, path, str, re, hash_map as map, hash_set as set };
pub use atoman::{ Lazy, lazy, State, StateGuard, Flag, Config, Logger, info, warn, error as err };

// Serde:
pub use serde::{ Serialize, Deserialize, de::DeserializeOwned };
pub use serde_json::{ self as json, json, Value as JsonValue };
// pub use toml::{ self, Value as TomlValue };

// STD:
pub use std::{
    format as fmt,
    fmt::Debug as Debugging,
    // collections::HashMap,
    path::{ Path, PathBuf },
    time::Duration,
    sync::{ Arc, Mutex as StdMutex },
    // time::std_sleep,
    // pin::Pin,
    // future::Future,
    net::SocketAddr,
};

// Tokio:
pub use tokio::{
    sync::Mutex,
    time::{ sleep, Instant },
    net::TcpListener,
};
