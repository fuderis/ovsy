// use crate::prelude::DynError;
use macron::{Display, Error, From};

// The error
#[derive(Debug, Display, Error, From)]
pub enum Error {
    #[display = "Failed to get agent name (incorrect dir path)"]
    FailedGetAgentName,
}
