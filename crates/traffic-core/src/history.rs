//! Client-local directional history navigation.

use std::collections::VecDeque;

use crate::{ForwardHistoryEntry, Game, GameCreated, GameEvent, GameSequence, HistoryEntry};

/// A position within surviving reversible history, independent of the
/// append-only canonical event log.
#[derive(Clone, Debug)]
pub struct HistoryCursor {
    game: Game,
    past: Vec<HistoryEntry>,
    future: VecDeque<ForwardHistoryEntry>,
    created: GameCreated,
    events: Vec<GameEvent>,
}

impl HistoryCursor {
    pub fn from_game(game: Game) -> Self {
        let past = game.history();
        Self {
            created: game.created.clone(),
            events: game.events.clone(),
            game,
            past,
            future: VecDeque::new(),
        }
    }

    pub fn game(&self) -> &Game {
        &self.game
    }
    pub fn is_live(&self) -> bool {
        self.future.is_empty()
    }

    pub fn step_back_batch(&mut self) -> bool {
        let mut moved = false;
        while let Some(entry) = self.past.pop() {
            let eviction = matches!(entry, HistoryEntry::Eviction(_));
            let _ = self.game.past.pop();
            self.future
                .push_front(self.game.unapply_history_entry(entry));
            moved = true;
            if !eviction {
                break;
            }
        }
        moved
    }

    pub fn step_forward_batch(&mut self) -> bool {
        let Some(entry) = self.future.pop_front() else {
            return false;
        };
        let past = self.game.apply_forward_history_entry(entry);
        self.game.past.push(past.clone());
        self.past.push(past);
        while matches!(self.future.front(), Some(ForwardHistoryEntry::Eviction(_))) {
            let entry = self.future.pop_front().expect("front was present");
            let past = self.game.apply_forward_history_entry(entry);
            self.game.past.push(past.clone());
            self.past.push(past);
        }
        true
    }

    /// Incorporates the next canonical event atomically, preserving review
    /// mode when possible and remaining live for live updates.
    pub fn apply_event(&mut self, event: GameEvent) -> bool {
        let expected = self
            .events
            .last()
            .map_or(GameSequence::INITIAL, GameEvent::sequence)
            .0
            .checked_add(1)
            .map(GameSequence);
        if expected != Some(event.sequence) {
            return false;
        }
        let was_live = self.is_live();
        let reviewed_entries = self.past.len();
        let mut events = self.events.clone();
        events.push(event);
        let mut game = Game::from_created(self.created.clone());
        if events
            .into_iter()
            .any(|event| game.apply_event(event).is_err())
        {
            return false;
        }
        let mut replacement = Self::from_game(game);
        if !was_live {
            while replacement.past.len() > reviewed_entries {
                if !replacement.step_back_batch() {
                    return false;
                }
            }
            if replacement.past.len() != reviewed_entries {
                return false;
            }
        }
        *self = replacement;
        true
    }
}
