//! Authoritative replay state and event orchestration.

use std::collections::BTreeMap;

use crate::*;

#[derive(Clone, Debug)]
pub struct TurnRecord {
    pub player: Owner,
    pub entries: Vec<HistoryEntry>,
}
#[derive(Clone, Debug)]
pub enum ApplyResult {
    Applied,
    Blocked(Vec<Violation>),
    Ended(Owner),
}

#[derive(Clone, Debug)]
pub struct Game {
    pub(crate) created: GameCreated,
    pub(crate) layout: Layout,
    sequence: GameSequence,
    pub(crate) events: Vec<GameEvent>,
    pub(crate) board: Board,
    pub(crate) active_player: Owner,
    pub(crate) active_card: Option<Card>,
    pub(crate) past: Vec<HistoryEntry>,
    records: Vec<TurnRecord>,
}

/// The complete locally replayed game state. `Game` remains an alias while the
/// CLI migrates to the replay-oriented name.
pub type GameState = Game;

impl Game {
    pub fn new(layout: Layout) -> Self {
        Self::from_created(GameCreated::new(layout))
    }

    pub fn from_created(created: GameCreated) -> Self {
        let layout = created.layout.clone();
        let mut cards = BTreeMap::new();
        for owner in [Owner::Heart, Owner::Spade] {
            let mut support = Support::Base(layout.start(owner));
            for value in (0..layout.rank_count()).rev() {
                let card = Card {
                    owner,
                    rank: Rank(value),
                };
                cards.insert(
                    card,
                    CardState {
                        support,
                        status: CardStatus::Idle,
                    },
                );
                support = Support::Card(card);
            }
        }
        Self {
            created,
            layout,
            sequence: GameSequence::INITIAL,
            events: Vec::new(),
            board: Board { cards },
            active_player: Owner::Heart,
            active_card: None,
            past: Vec::new(),
            records: Vec::new(),
        }
    }
    pub fn layout(&self) -> &Layout {
        &self.layout
    }
    pub const fn created(&self) -> &GameCreated {
        &self.created
    }
    pub const fn sequence(&self) -> GameSequence {
        self.sequence
    }
    pub fn events(&self) -> &[GameEvent] {
        &self.events
    }
    pub fn board(&self) -> &Board {
        &self.board
    }
    pub const fn active_player(&self) -> Owner {
        self.active_player
    }
    pub const fn active_card(&self) -> Option<Card> {
        self.active_card
    }
    pub fn winner(&self) -> Option<Owner> {
        (self.active_card.is_none() && self.complete(self.active_player).ok()?)
            .then_some(self.active_player)
    }
    pub fn blocked(&self) -> Option<Vec<Violation>> {
        let entry = self
            .past
            .iter()
            .rev()
            .find(|entry| !matches!(entry, HistoryEntry::Eviction(_)))?;
        let violations = match entry {
            HistoryEntry::Root(card) => {
                let mut result = Vec::new();
                if card.owner != self.active_player {
                    result.push(Violation::RootOwner);
                }
                for (other, state) in self.board.cards() {
                    if other.owner == self.active_player
                        && other != *card
                        && state.status == CardStatus::Evicted
                    {
                        result.push(Violation::ActiveEviction(other));
                    }
                }
                result
            }
            HistoryEntry::Intention(card) => self
                .validate_intention_position(*card)
                .unwrap_or_else(|_| vec![Violation::NotEvicted(*card)]),
            HistoryEntry::Fulfilment { card, .. } => self.fulfilment_violations(*card),
            HistoryEntry::End => self
                .board
                .cards
                .values()
                .any(|state| state.status != CardStatus::Idle)
                .then_some(Violation::PendingCards)
                .into_iter()
                .collect(),
            HistoryEntry::Eviction(_) => Vec::new(),
        };
        (!violations.is_empty()).then_some(violations)
    }
    pub fn history(&self) -> Vec<HistoryEntry> {
        self.past.clone()
    }
    pub fn records(&self) -> &[TurnRecord] {
        &self.records
    }

    pub fn apply(&mut self, operation: Operation) -> Result<ApplyResult, CoreError> {
        self.apply_delta(GameDelta::Operation(operation))
            .map(|(_, outcome)| outcome)
    }

    pub fn apply_delta(&mut self, delta: GameDelta) -> Result<(GameEvent, ApplyResult), CoreError> {
        let sequence = self.next_sequence()?;
        let before = self.clone();
        let outcome = match self.apply_delta_inner(delta.clone()) {
            Ok(outcome) => outcome,
            Err(error) => {
                *self = before;
                return Err(error);
            }
        };
        let event = GameEvent::new(sequence, delta);
        self.sequence = sequence;
        self.events.push(event.clone());
        Ok((event, outcome))
    }

    pub fn apply_event(&mut self, event: GameEvent) -> Result<ApplyResult, CoreError> {
        if event.sequence != self.next_sequence()? {
            return Err(CoreError::new(
                "game event sequence is not the next sequence",
            ));
        }
        let before = self.clone();
        let outcome = match self.apply_delta_inner(event.delta.clone()) {
            Ok(outcome) => outcome,
            Err(error) => {
                *self = before;
                return Err(error);
            }
        };
        self.sequence = event.sequence;
        self.events.push(event);
        Ok(outcome)
    }

    fn apply_delta_inner(&mut self, delta: GameDelta) -> Result<ApplyResult, CoreError> {
        match delta {
            GameDelta::Operation(operation) => self.apply_operation(operation),
            GameDelta::Undo => self.undo_batch(),
        }
    }

    fn apply_operation(&mut self, operation: Operation) -> Result<ApplyResult, CoreError> {
        if self.winner().is_some() {
            return Err(CoreError::new("game has ended"));
        }
        if self.blocked().is_some() {
            return Err(CoreError::new("current batch is blocked; undo first"));
        }
        self.apply_operation_entry(operation)?;
        if let Some(violations) = self.blocked() {
            return Ok(ApplyResult::Blocked(violations));
        }
        self.refresh_records();
        Ok(self
            .winner()
            .map_or(ApplyResult::Applied, ApplyResult::Ended))
    }

    pub fn undo(&mut self) -> Result<ApplyResult, CoreError> {
        self.apply_delta(GameDelta::Undo)
            .map(|(_, outcome)| outcome)
    }

    fn undo_batch(&mut self) -> Result<ApplyResult, CoreError> {
        if self.past.is_empty() {
            return Err(CoreError::new("there is no batch to undo"));
        }
        loop {
            let entry = self
                .past
                .pop()
                .ok_or_else(|| CoreError::new("history is inconsistent"))?;
            let eviction = matches!(entry, HistoryEntry::Eviction(_));
            self.unapply_history_entry(entry);
            if !eviction {
                break;
            }
        }
        self.refresh_records();
        Ok(ApplyResult::Applied)
    }

    pub(crate) fn apply_forward_history_entry(
        &mut self,
        entry: ForwardHistoryEntry,
    ) -> HistoryEntry {
        match entry {
            ForwardHistoryEntry::Root(card) => {
                self.active_card = Some(card);
                let state = self.board.cards.get_mut(&card).expect("known history card");
                state.status = CardStatus::Evicted;
                HistoryEntry::Root(card)
            }
            ForwardHistoryEntry::Eviction(card) => {
                let state = self.board.cards.get_mut(&card).expect("known history card");
                state.status = CardStatus::Evicted;
                HistoryEntry::Eviction(card)
            }
            ForwardHistoryEntry::Intention { card, target } => {
                let state = self.board.cards.get_mut(&card).expect("known history card");
                state.status = CardStatus::Intended(target);
                HistoryEntry::Intention(card)
            }
            ForwardHistoryEntry::Fulfilment(card) => {
                let state = self.board.cards.get_mut(&card).expect("known history card");
                let source = state.support;
                let CardStatus::Intended(target) = state.status else {
                    unreachable!("forward fulfilment must follow its intention");
                };
                state.support = target;
                state.status = CardStatus::Idle;
                HistoryEntry::Fulfilment { source, card }
            }
            ForwardHistoryEntry::End => {
                self.active_card = None;
                self.active_player = self.active_player.other();
                HistoryEntry::End
            }
        }
    }

    pub(crate) fn unapply_history_entry(&mut self, entry: HistoryEntry) -> ForwardHistoryEntry {
        match entry {
            HistoryEntry::Root(card) => {
                self.board
                    .cards
                    .get_mut(&card)
                    .expect("known history card")
                    .status = CardStatus::Idle;
                self.active_card = None;
                ForwardHistoryEntry::Root(card)
            }
            HistoryEntry::Eviction(card) => {
                self.board
                    .cards
                    .get_mut(&card)
                    .expect("known history card")
                    .status = CardStatus::Idle;
                ForwardHistoryEntry::Eviction(card)
            }
            HistoryEntry::Intention(card) => {
                let state = self.board.cards.get_mut(&card).expect("known history card");
                let CardStatus::Intended(target) = state.status else {
                    unreachable!("reverse intention must be currently intended");
                };
                state.status = CardStatus::Evicted;
                ForwardHistoryEntry::Intention { card, target }
            }
            HistoryEntry::Fulfilment { source, card } => {
                let state = self.board.cards.get_mut(&card).expect("known history card");
                let target = state.support;
                state.support = source;
                state.status = CardStatus::Intended(target);
                ForwardHistoryEntry::Fulfilment(card)
            }
            HistoryEntry::End => {
                self.active_player = self.active_player.other();
                self.active_card = self
                    .past
                    .iter()
                    .rev()
                    .take_while(|entry| !matches!(entry, HistoryEntry::End))
                    .find_map(|entry| match entry {
                        HistoryEntry::Root(card) => Some(*card),
                        _ => None,
                    });
                ForwardHistoryEntry::End
            }
        }
    }

    fn next_sequence(&self) -> Result<GameSequence, CoreError> {
        self.sequence
            .0
            .checked_add(1)
            .map(GameSequence)
            .ok_or_else(|| CoreError::new("game event sequence overflow"))
    }

    fn refresh_records(&mut self) {
        let mut player = Owner::Heart;
        let mut start = 0;
        self.records.clear();
        for (index, entry) in self.past.iter().enumerate() {
            if matches!(entry, HistoryEntry::End) {
                self.records.push(TurnRecord {
                    player,
                    entries: self.past[start..=index].to_vec(),
                });
                player = player.other();
                start = index + 1;
            }
        }
    }
}
