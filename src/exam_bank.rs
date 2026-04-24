use crate::deck::Deck;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExamBank {
    pub(crate) title: String,
    pub(crate) decks: Vec<Deck>,
}
