use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Card {
    pub(crate) question: String,
    pub(crate) answer: String,
}
