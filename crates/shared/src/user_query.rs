use anylm::Message;
use serde::{Deserialize, Serialize};

/// The user query data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserQuery {
    pub user_id: u128,
    pub messages: Vec<Message>,
}
