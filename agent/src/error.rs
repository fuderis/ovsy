use macron::{Display, Error, From, prelude::DynError};

// The error
#[derive(Debug, Display, Error, From)]
pub enum Error {
    #[from]
    Io(std::io::Error),

    #[from]
    String(String),

    #[display = "Agent execution error: {0}"]
    ExecutionStop(Box<DynError>),
}
