// use crate::prelude::DynError;
use macron::{Display, Error, From};

// The error
#[derive(Debug, Display, Error, From)]
pub enum Error {
    #[display(fmt = "A non-existing tool was called: `{0}`")]
    UnknownTool(String),

    #[cfg(target_os = "windows")]
    #[from(skip)]
    #[display(fmt = "Task join error: {0}")]
    TaskJoin(DynError),

    #[cfg(target_os = "windows")]
    #[display(fmt = "Failed to write Windows registry: {0}")]
    Registry(std::io::Error),
}
