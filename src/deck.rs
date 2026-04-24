use crate::card::Card;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Deck {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) cards: Vec<Card>,
}
