use crate::session_action::SessionAction;
use crate::session_card::SessionCard;
use anyhow::bail;
use rand::prelude::SliceRandom;

#[derive(Debug)]
pub struct SessionState {
    queue: Vec<SessionCard>,
    pub(crate) total: usize,
    pub(crate) mastered: usize,
    pub(crate) answer_shown: bool,
    pub(crate) finished: bool,
    pub(crate) aborted: bool,
}

impl SessionState {
    pub(crate) fn new(mut cards: Vec<SessionCard>) -> anyhow::Result<Self> {
        if cards.is_empty() {
            bail!("банк вопросов пуст");
        }

        let mut rng = rand::rng();
        cards.shuffle(&mut rng);

        let total = cards.len();
        Ok(Self {
            queue: cards,
            total,
            mastered: 0,
            answer_shown: false,
            finished: false,
            aborted: false,
        })
    }

    pub(crate) fn current(&self) -> Option<&SessionCard> {
        self.queue.first()
    }

    pub(crate) fn remaining(&self) -> usize {
        self.queue.len()
    }

    pub(crate) fn apply(&mut self, action: SessionAction) {
        match action {
            SessionAction::Show if !self.answer_shown => {
                self.answer_shown = true;
            }
            SessionAction::Know => {
                if !self.queue.is_empty() {
                    self.queue.remove(0);
                    self.mastered += 1;
                    self.answer_shown = false;
                }
            }
            SessionAction::DontKnow => {
                if !self.queue.is_empty() {
                    let card = self.queue.remove(0);
                    self.queue.push(card);
                    let mut rng = rand::rng();
                    self.queue.shuffle(&mut rng);
                    self.answer_shown = false;
                }
            }
            SessionAction::Quit => {
                self.aborted = true;
                self.finished = true;
            }
            SessionAction::Show => {}
        }

        if self.queue.is_empty() {
            self.finished = true;
        }
    }
}
