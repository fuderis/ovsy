use serde::{Deserialize, Serialize};

/// The chat action
#[derive(Serialize, Deserialize, Eq, PartialEq)]
pub enum ChatAction {
    Query(String),
    Cancel,
}
