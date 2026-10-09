//! Deterministic rule transitions and incremental validation.

use crate::{
    Card, CardStatus, CoreError, Game, HistoryEntry, Operation, Owner, Support, Violation,
};

impl Game {
    pub(crate) fn apply_operation_entry(&mut self, operation: Operation) -> Result<(), CoreError> {
        match operation {
            Operation::Root(card) => {
                self.board
                    .card_state(card)
                    .ok_or_else(|| CoreError::new(format!("unknown card {card}")))?;
                self.active_card = Some(card);
                self.board
                    .cards
                    .get_mut(&card)
                    .ok_or_else(|| CoreError::new("unknown root card"))?
                    .status = CardStatus::Evicted;
                self.past.push(HistoryEntry::Root(card));
                let cover = self.board.occupants(Support::Card(card)).next();
                if let Some(cover) = cover {
                    self.append_eviction_chain(cover)?;
                }
            }
            Operation::Intention { card, target } => {
                self.board
                    .card_state(card)
                    .ok_or_else(|| CoreError::new(format!("unknown card {card}")))?;
                if let Support::Card(target_card) = target {
                    self.board.card_state(target_card).ok_or_else(|| {
                        CoreError::new(format!("unknown support card {target_card}"))
                    })?;
                }
                self.board
                    .cards
                    .get_mut(&card)
                    .ok_or_else(|| CoreError::new(format!("unknown card {card}")))?
                    .status = CardStatus::Intended(target);
                self.past.push(HistoryEntry::Intention(card));
                let occupant = self
                    .board
                    .occupants(target)
                    .find(|occupant| *occupant != card);
                if let Some(occupant) = occupant {
                    self.append_eviction_chain(occupant)?;
                }
            }
            Operation::Fulfilment(card) => {
                let state = self
                    .board
                    .card_state(card)
                    .cloned()
                    .ok_or_else(|| CoreError::new(format!("unknown card {card}")))?;
                let CardStatus::Intended(target) = state.status else {
                    return Err(CoreError::new(format!("{card} is not intended")));
                };
                let card_state = self
                    .board
                    .cards
                    .get_mut(&card)
                    .ok_or_else(|| CoreError::new(format!("unknown card {card}")))?;
                card_state.support = target;
                card_state.status = CardStatus::Idle;
                self.past.push(HistoryEntry::Fulfilment {
                    source: state.support,
                    card,
                });
                if matches!(target, Support::Card(lower) if lower.owner != card.owner) {
                    self.append_eviction_chain(card)?;
                }
            }
            Operation::End => {
                self.past.push(HistoryEntry::End);
                self.active_card = None;
                self.active_player = self.active_player.other();
            }
        }
        Ok(())
    }

    pub(crate) fn validate_intention_position(
        &self,
        card: Card,
    ) -> Result<Vec<Violation>, CoreError> {
        let state = self
            .board
            .cards
            .get(&card)
            .ok_or_else(|| CoreError::new(format!("unknown card {card}")))?;
        let mut violations = Vec::new();
        let CardStatus::Intended(target) = state.status else {
            violations.push(Violation::NotEvicted(card));
            return Ok(violations);
        };
        if self.board.is_occupied(Support::Card(card)) {
            violations.push(Violation::CoveredMover(card));
        }
        if card.owner == self.active_player && self.active_card != Some(card) {
            violations.push(Violation::NonActiveOwner(card));
        }
        let source = self.board.square_of(card)?;
        let target_square = self.board.support_square(target)?;
        if !self.layout.contains(target_square) {
            violations.push(Violation::UnknownTargetSquare(target_square));
        }
        if !self.layout.adjacent(source, target_square) {
            violations.push(Violation::NonAdjacent);
        }
        if self.layout.is_foreign_special(card.owner, target_square) {
            violations.push(Violation::ForeignSpecialSquare);
        }
        if let Support::Card(lower) = target {
            if card.rank > lower.rank {
                violations.push(Violation::LargerOnSmaller);
            }
            if self.board.cards[&lower].status != CardStatus::Idle {
                violations.push(Violation::SupportCollision(lower));
            }
        }
        if let Some(occupant) = self
            .board
            .occupants(target)
            .find(|occupant| *occupant != card)
            && occupant.rank >= card.rank
        {
            violations.push(Violation::CannotPushEqualOrLarger);
        }
        for (other, state) in self.board.cards() {
            if other != card
                && let CardStatus::Intended(other_target) = state.status
            {
                if other_target == target {
                    violations.push(Violation::TargetCollision(target));
                }
                if target == Support::Card(other) {
                    violations.push(Violation::SupportCollision(other));
                }
            }
        }
        Ok(violations)
    }

    pub(crate) fn fulfilment_violations(&self, card: Card) -> Vec<Violation> {
        let Some(state) = self.board.cards.get(&card) else {
            return vec![Violation::NotIntended(card)];
        };
        let mut violations = Vec::new();
        if self.board.is_occupied(Support::Card(card)) {
            violations.push(Violation::CoveredMover(card));
        }
        if self
            .board
            .occupants(state.support)
            .any(|occupant| occupant != card)
        {
            violations.push(Violation::OccupiedTarget(state.support));
        }
        violations
    }

    fn append_eviction_chain(&mut self, card: Card) -> Result<(), CoreError> {
        if self.board.cards[&card].status != CardStatus::Idle {
            return Ok(());
        }
        self.board
            .cards
            .get_mut(&card)
            .ok_or_else(|| CoreError::new(format!("unknown card {card}")))?
            .status = CardStatus::Evicted;
        self.past.push(HistoryEntry::Eviction(card));
        let cover = self.board.occupants(Support::Card(card)).next();
        match cover {
            Some(cover) => self.append_eviction_chain(cover),
            None => Ok(()),
        }
    }

    pub(crate) fn complete(&self, owner: Owner) -> Result<bool, CoreError> {
        let finish = self.layout.finish(owner);
        self.board
            .cards()
            .filter(|(card, _)| card.owner == owner)
            .try_fold(true, |_, (card, _)| {
                Ok(self.board.square_of(card)? == finish)
            })
    }
}
