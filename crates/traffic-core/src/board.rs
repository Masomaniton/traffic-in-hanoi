//! Direct-support board representation and derived board queries.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{Card, CoreError, Square, Support};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CardStatus {
    Idle,
    Evicted,
    Intended(Support),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CardState {
    pub support: Support,
    pub status: CardStatus,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Board {
    pub(crate) cards: BTreeMap<Card, CardState>,
}

impl Board {
    pub fn card_state(&self, card: Card) -> Option<&CardState> {
        self.cards.get(&card)
    }
    pub fn cards(&self) -> impl Iterator<Item = (Card, &CardState)> {
        self.cards.iter().map(|(card, state)| (*card, state))
    }

    pub fn square_of(&self, card: Card) -> Result<Square, CoreError> {
        let mut seen = BTreeSet::new();
        let mut support = self
            .cards
            .get(&card)
            .ok_or_else(|| CoreError::new(format!("unknown card {card}")))?
            .support;
        loop {
            match support {
                Support::Base(square) => return Ok(square),
                Support::Card(lower) => {
                    if !seen.insert(lower) {
                        return Err(CoreError::new("support cycle detected"));
                    }
                    support = self
                        .cards
                        .get(&lower)
                        .ok_or_else(|| CoreError::new(format!("unknown support card {lower}")))?
                        .support;
                }
            }
        }
    }

    pub fn support_square(&self, support: Support) -> Result<Square, CoreError> {
        match support {
            Support::Base(square) => Ok(square),
            Support::Card(card) => self.square_of(card),
        }
    }

    pub fn occupants(&self, support: Support) -> impl Iterator<Item = Card> + '_ {
        self.cards
            .iter()
            .filter_map(move |(card, state)| (state.support == support).then_some(*card))
    }

    pub fn is_occupied(&self, support: Support) -> bool {
        self.occupants(support).next().is_some()
    }
}
