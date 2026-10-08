use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Square {
    pub column: u16,
    pub row: u16,
}

impl Square {
    pub const fn new(column: u16, row: u16) -> Self {
        Self { column, row }
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.column, self.row)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Owner {
    Heart,
    Spade,
}

impl Owner {
    pub const fn other(self) -> Self {
        match self {
            Self::Heart => Self::Spade,
            Self::Spade => Self::Heart,
        }
    }
    pub const fn symbol(self) -> char {
        match self {
            Self::Heart => 'H',
            Self::Spade => 'S',
        }
    }
}

impl fmt::Display for Owner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Heart => "Heart",
            Self::Spade => "Spade",
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Rank(u16);

impl Rank {
    pub fn new(value: u16, rank_count: u16) -> Result<Self, CoreError> {
        (value < rank_count)
            .then_some(Self(value))
            .ok_or_else(|| CoreError::new("rank is outside the layout's range"))
    }
    pub const fn value(self) -> u16 {
        self.0
    }
    pub const fn display(self) -> u16 {
        self.0 + 1
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Card {
    pub owner: Owner,
    pub rank: Rank,
}

impl Card {
    pub fn from_display(owner: Owner, displayed: u16, rank_count: u16) -> Result<Self, CoreError> {
        let value = displayed
            .checked_sub(1)
            .ok_or_else(|| CoreError::new("displayed rank must be at least 1"))?;
        Ok(Self {
            owner,
            rank: Rank::new(value, rank_count)?,
        })
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.owner.symbol(), self.rank.display())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Support {
    Base(Square),
    Card(Card),
}

impl fmt::Display for Support {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Base(square) => write!(f, "base{square}"),
            Self::Card(card) => write!(f, "{card}"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Layout {
    rank_count: u16,
    squares: BTreeSet<Square>,
    starts: BTreeMap<Owner, Square>,
    finishes: BTreeMap<Owner, Square>,
}

impl Layout {
    pub fn new(
        rank_count: u16,
        squares: BTreeSet<Square>,
        starts: BTreeMap<Owner, Square>,
        finishes: BTreeMap<Owner, Square>,
    ) -> Result<Self, CoreError> {
        if rank_count == 0 || squares.is_empty() {
            return Err(CoreError::new(
                "rank count and square set must be non-empty",
            ));
        }
        if squares.iter().map(|s| s.column).min() != Some(0)
            || squares.iter().map(|s| s.row).min() != Some(0)
        {
            return Err(CoreError::new(
                "layout coordinates must be normalized to zero",
            ));
        }
        for owner in [Owner::Heart, Owner::Spade] {
            let start = starts
                .get(&owner)
                .ok_or_else(|| CoreError::new("missing start square"))?;
            let finish = finishes
                .get(&owner)
                .ok_or_else(|| CoreError::new("missing finish square"))?;
            if !squares.contains(start) || !squares.contains(finish) {
                return Err(CoreError::new("special squares must belong to the layout"));
            }
        }
        let special_squares = [
            starts[&Owner::Heart],
            finishes[&Owner::Heart],
            starts[&Owner::Spade],
            finishes[&Owner::Spade],
        ];
        if special_squares.into_iter().collect::<BTreeSet<_>>().len() != special_squares.len() {
            return Err(CoreError::new("special squares must be distinct"));
        }
        Ok(Self {
            rank_count,
            squares,
            starts,
            finishes,
        })
    }

    pub fn standard(rank_count: u16) -> Result<Self, CoreError> {
        let mut squares = BTreeSet::new();
        for column in 0..=2 {
            for row in 0..=4 {
                if column != 1 || !matches!(row, 0 | 4) {
                    squares.insert(Square::new(column, row));
                }
            }
        }
        Self::new(
            rank_count,
            squares,
            BTreeMap::from([
                (Owner::Heart, Square::new(0, 0)),
                (Owner::Spade, Square::new(0, 4)),
            ]),
            BTreeMap::from([
                (Owner::Heart, Square::new(2, 4)),
                (Owner::Spade, Square::new(2, 0)),
            ]),
        )
    }

    pub const fn rank_count(&self) -> u16 {
        self.rank_count
    }
    pub fn squares(&self) -> impl Iterator<Item = Square> + '_ {
        self.squares.iter().copied()
    }
    pub fn start(&self, owner: Owner) -> Square {
        self.starts[&owner]
    }
    pub fn finish(&self, owner: Owner) -> Square {
        self.finishes[&owner]
    }
    pub fn contains(&self, square: Square) -> bool {
        self.squares.contains(&square)
    }
    pub fn adjacent(&self, a: Square, b: Square) -> bool {
        a.column.abs_diff(b.column) + a.row.abs_diff(b.row) == 1
    }
    pub fn is_foreign_special(&self, owner: Owner, square: Square) -> bool {
        square == self.start(owner.other()) || square == self.finish(owner.other())
    }
}

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
    cards: BTreeMap<Card, CardState>,
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
    pub fn occupant(&self, support: Support) -> Option<Card> {
        self.cards
            .iter()
            .find_map(|(card, state)| (state.support == support).then_some(*card))
    }
    pub fn pile(&self, square: Square) -> Vec<Card> {
        let mut pile = Vec::new();
        let mut support = Support::Base(square);
        while let Some(card) = self.occupant(support) {
            pile.push(card);
            support = Support::Card(card);
        }
        pile
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Operation {
    Root(Card),
    Intention { card: Card, target: Support },
    Fulfilment(Card),
    End,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryEntry {
    Root(Card),
    Intention { card: Card, target: Support },
    Fulfilment { card: Card, target: Support },
    End,
    Eviction(Card),
}

impl fmt::Display for HistoryEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Root(card) => write!(f, "Root({card})"),
            Self::Intention { card, target } => write!(f, "Intention({card} -> {target})"),
            Self::Fulfilment { card, target } => write!(f, "Fulfilment({card} -> {target})"),
            Self::End => f.write_str("End"),
            Self::Eviction(card) => write!(f, "Eviction({card})"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Violation {
    RootOwner,
    RootAlreadySelected,
    ActiveEviction(Card),
    NotEvicted(Card),
    CoveredMover(Card),
    NonActiveOwner(Card),
    NonAdjacent,
    UnknownTargetSquare(Square),
    ForeignSpecialSquare,
    LargerOnSmaller,
    CannotPushEqualOrLarger,
    TargetCollision(Support),
    SupportCollision(Card),
    NotIntended(Card),
    OccupiedTarget(Support),
    PendingCards,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RootOwner => f.write_str("root card must belong to the active player"),
            Self::RootAlreadySelected => f.write_str("a root card is already selected"),
            Self::ActiveEviction(card) | Self::NonActiveOwner(card) => {
                write!(f, "{card} is owned by the active player but is not active")
            }
            Self::NotEvicted(card) => write!(f, "{card} is not evicted"),
            Self::CoveredMover(card) => write!(f, "{card} has a card on top"),
            Self::NonAdjacent => f.write_str("source and target are not adjacent"),
            Self::UnknownTargetSquare(square) => {
                write!(f, "target square {square} does not belong to the layout")
            }
            Self::ForeignSpecialSquare => {
                f.write_str("card cannot enter the opponent's special square")
            }
            Self::LargerOnSmaller => f.write_str("card cannot rest on a lower-ranked card"),
            Self::CannotPushEqualOrLarger => {
                f.write_str("card cannot push an equal- or greater-ranked card")
            }
            Self::TargetCollision(support) => {
                write!(f, "another intention already targets {support}")
            }
            Self::SupportCollision(card) => write!(f, "target support {card} is not idle"),
            Self::NotIntended(card) => write!(f, "{card} is not intended"),
            Self::OccupiedTarget(support) => write!(f, "target support {support} is occupied"),
            Self::PendingCards => f.write_str("cannot end while cards are evicted or intended"),
        }
    }
}

#[derive(Clone, Debug)]
struct Snapshot {
    board: Board,
    active_card: Option<Card>,
}
#[derive(Clone, Debug)]
struct Batch {
    before: Snapshot,
    entries: Vec<HistoryEntry>,
    blocked: Option<Vec<Violation>>,
}
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
    layout: Layout,
    board: Board,
    active_player: Owner,
    active_card: Option<Card>,
    batches: Vec<Batch>,
    records: Vec<TurnRecord>,
    winner: Option<Owner>,
}

impl Game {
    pub fn new(layout: Layout) -> Self {
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
            layout,
            board: Board { cards },
            active_player: Owner::Heart,
            active_card: None,
            batches: Vec::new(),
            records: Vec::new(),
            winner: None,
        }
    }
    pub fn layout(&self) -> &Layout {
        &self.layout
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
    pub const fn winner(&self) -> Option<Owner> {
        self.winner
    }
    pub fn blocked(&self) -> Option<&[Violation]> {
        self.batches
            .last()
            .and_then(|batch| batch.blocked.as_deref())
    }
    pub fn history(&self) -> Vec<HistoryEntry> {
        self.batches
            .iter()
            .flat_map(|batch| batch.entries.iter().cloned())
            .collect()
    }
    pub fn records(&self) -> &[TurnRecord] {
        &self.records
    }

    pub fn apply(&mut self, operation: Operation) -> Result<ApplyResult, CoreError> {
        if self.winner.is_some() {
            return Err(CoreError::new("game has ended"));
        }
        if self.blocked().is_some() {
            return Err(CoreError::new("current batch is blocked; undo first"));
        }
        let before = Snapshot {
            board: self.board.clone(),
            active_card: self.active_card,
        };
        let mut batch = Batch {
            before,
            entries: Vec::new(),
            blocked: None,
        };
        let violations = match operation.clone() {
            Operation::Root(card) => self.apply_root(card, &mut batch)?,
            Operation::Intention { card, target } => {
                self.apply_intention(card, target, &mut batch)?
            }
            Operation::Fulfilment(card) => self.apply_fulfilment(card, &mut batch)?,
            Operation::End => self.apply_end(&mut batch),
        };
        if !violations.is_empty() {
            batch.blocked = Some(violations.clone());
            self.batches.push(batch);
            return Ok(ApplyResult::Blocked(violations));
        }
        if matches!(operation, Operation::End) {
            let mut entries = self.history();
            entries.extend(batch.entries);
            self.records.push(TurnRecord {
                player: self.active_player,
                entries,
            });
            self.batches.clear();
            self.active_card = None;
            self.active_player = self.active_player.other();
            if self.complete(self.active_player)? {
                self.winner = Some(self.active_player);
                return Ok(ApplyResult::Ended(self.active_player));
            }
            return Ok(ApplyResult::Applied);
        }
        self.batches.push(batch);
        Ok(ApplyResult::Applied)
    }

    pub fn undo(&mut self) -> bool {
        let Some(batch) = self.batches.pop() else {
            return false;
        };
        self.board = batch.before.board;
        self.active_card = batch.before.active_card;
        true
    }

    fn apply_root(&mut self, card: Card, batch: &mut Batch) -> Result<Vec<Violation>, CoreError> {
        let mut violations = Vec::new();
        if self.active_card.is_some() {
            violations.push(Violation::RootAlreadySelected);
        }
        if card.owner != self.active_player {
            violations.push(Violation::RootOwner);
        }
        batch.entries.push(HistoryEntry::Root(card));
        if !violations.is_empty() {
            return Ok(violations);
        }
        self.active_card = Some(card);
        self.board
            .cards
            .get_mut(&card)
            .ok_or_else(|| CoreError::new("unknown root card"))?
            .status = CardStatus::Evicted;
        self.after_eviction(card, batch)
    }
    fn apply_intention(
        &mut self,
        card: Card,
        target: Support,
        batch: &mut Batch,
    ) -> Result<Vec<Violation>, CoreError> {
        let violations = self.validate_intention(card, target)?;
        batch.entries.push(HistoryEntry::Intention { card, target });
        if !violations.is_empty() {
            return Ok(violations);
        }
        self.board
            .cards
            .get_mut(&card)
            .ok_or_else(|| CoreError::new(format!("unknown card {card}")))?
            .status = CardStatus::Intended(target);
        match self.board.occupant(target) {
            Some(occupant) => self.append_eviction(occupant, batch),
            None => Ok(Vec::new()),
        }
    }
    fn apply_fulfilment(
        &mut self,
        card: Card,
        batch: &mut Batch,
    ) -> Result<Vec<Violation>, CoreError> {
        let (target, violations) = self.validate_fulfilment(card)?;
        batch
            .entries
            .push(HistoryEntry::Fulfilment { card, target });
        if !violations.is_empty() {
            return Ok(violations);
        }
        let state = self
            .board
            .cards
            .get_mut(&card)
            .ok_or_else(|| CoreError::new(format!("unknown card {card}")))?;
        state.support = target;
        state.status = CardStatus::Idle;
        if let Support::Card(lower) = target
            && lower.owner != card.owner
        {
            return self.append_eviction(card, batch);
        }
        Ok(Vec::new())
    }
    fn apply_end(&self, batch: &mut Batch) -> Vec<Violation> {
        batch.entries.push(HistoryEntry::End);
        if self
            .board
            .cards
            .values()
            .any(|state| state.status != CardStatus::Idle)
        {
            vec![Violation::PendingCards]
        } else {
            Vec::new()
        }
    }

    fn validate_intention(&self, card: Card, target: Support) -> Result<Vec<Violation>, CoreError> {
        let state = self
            .board
            .cards
            .get(&card)
            .ok_or_else(|| CoreError::new(format!("unknown card {card}")))?;
        let mut v = Vec::new();
        if state.status != CardStatus::Evicted {
            v.push(Violation::NotEvicted(card));
        }
        if self.board.occupant(Support::Card(card)).is_some() {
            v.push(Violation::CoveredMover(card));
        }
        if card.owner == self.active_player && self.active_card != Some(card) {
            v.push(Violation::NonActiveOwner(card));
        }
        let source = self.board.square_of(card)?;
        let target_square = self.board.support_square(target)?;
        if !self.layout.contains(target_square) {
            v.push(Violation::UnknownTargetSquare(target_square));
        }
        if !self.layout.adjacent(source, target_square) {
            v.push(Violation::NonAdjacent);
        }
        if self.layout.is_foreign_special(card.owner, target_square) {
            v.push(Violation::ForeignSpecialSquare);
        }
        if let Support::Card(lower) = target {
            if card.rank > lower.rank {
                v.push(Violation::LargerOnSmaller);
            }
            if self.board.cards[&lower].status != CardStatus::Idle {
                v.push(Violation::SupportCollision(lower));
            }
        }
        if let Some(occupant) = self.board.occupant(target)
            && occupant.rank >= card.rank
        {
            v.push(Violation::CannotPushEqualOrLarger);
        }
        for (other, state) in self.board.cards() {
            if let CardStatus::Intended(other_target) = state.status {
                if other_target == target {
                    v.push(Violation::TargetCollision(target));
                }
                if target == Support::Card(other) {
                    v.push(Violation::SupportCollision(other));
                }
            }
        }
        Ok(v)
    }
    fn validate_fulfilment(&self, card: Card) -> Result<(Support, Vec<Violation>), CoreError> {
        let state = self
            .board
            .cards
            .get(&card)
            .ok_or_else(|| CoreError::new(format!("unknown card {card}")))?;
        let target = match state.status {
            CardStatus::Intended(target) => target,
            _ => {
                return Ok((
                    Support::Base(self.board.square_of(card)?),
                    vec![Violation::NotIntended(card)],
                ));
            }
        };
        let mut v = Vec::new();
        if self.board.occupant(Support::Card(card)).is_some() {
            v.push(Violation::CoveredMover(card));
        }
        if self.board.occupant(target).is_some() {
            v.push(Violation::OccupiedTarget(target));
        }
        Ok((target, v))
    }
    fn append_eviction(
        &mut self,
        card: Card,
        batch: &mut Batch,
    ) -> Result<Vec<Violation>, CoreError> {
        if self.board.cards[&card].status != CardStatus::Idle {
            return Ok(Vec::new());
        }
        self.board
            .cards
            .get_mut(&card)
            .ok_or_else(|| CoreError::new(format!("unknown card {card}")))?
            .status = CardStatus::Evicted;
        batch.entries.push(HistoryEntry::Eviction(card));
        self.after_eviction(card, batch)
    }
    fn after_eviction(
        &mut self,
        card: Card,
        batch: &mut Batch,
    ) -> Result<Vec<Violation>, CoreError> {
        let mut v = Vec::new();
        if card.owner == self.active_player && self.active_card != Some(card) {
            v.push(Violation::ActiveEviction(card));
        }
        for (_, state) in self.board.cards() {
            if let CardStatus::Intended(Support::Card(support)) = state.status
                && support == card
            {
                v.push(Violation::SupportCollision(card));
            }
        }
        if !v.is_empty() {
            return Ok(v);
        }
        match self.board.occupant(Support::Card(card)) {
            Some(cover) => self.append_eviction(cover, batch),
            None => Ok(Vec::new()),
        }
    }
    fn complete(&self, owner: Owner) -> Result<bool, CoreError> {
        let finish = self.layout.finish(owner);
        self.board
            .cards()
            .filter(|(card, _)| card.owner == owner)
            .try_fold(true, |_, (card, _)| {
                Ok(self.board.square_of(card)? == finish)
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoreError(String);
impl CoreError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}
impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for CoreError {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configurable_cards_initialize() {
        let game = Game::new(Layout::standard(3).unwrap());
        assert_eq!(game.board.cards().count(), 6);
        let top = Card::from_display(Owner::Heart, 1, 3).unwrap();
        assert_eq!(game.board.square_of(top).unwrap(), Square::new(0, 0));
    }
    #[test]
    fn root_undo_restores_turn_start() {
        let mut game = Game::new(Layout::standard(3).unwrap());
        let card = Card::from_display(Owner::Heart, 1, 3).unwrap();
        assert!(matches!(
            game.apply(Operation::Root(card)).unwrap(),
            ApplyResult::Applied
        ));
        assert!(game.undo());
        assert!(game.history().is_empty());
    }

    #[test]
    fn buried_root_becomes_a_blocked_batch_and_undo_restores_everything() {
        let mut game = Game::new(Layout::standard(3).unwrap());
        let buried = Card::from_display(Owner::Heart, 2, 3).unwrap();
        let top = Card::from_display(Owner::Heart, 1, 3).unwrap();

        assert!(matches!(
            game.apply(Operation::Root(buried)).unwrap(),
            ApplyResult::Blocked(violations) if violations == vec![Violation::ActiveEviction(top)]
        ));
        assert_eq!(
            game.board.card_state(buried).unwrap().status,
            CardStatus::Evicted
        );
        assert_eq!(
            game.board.card_state(top).unwrap().status,
            CardStatus::Evicted
        );

        assert!(game.undo());
        assert_eq!(
            game.board.card_state(buried).unwrap().status,
            CardStatus::Idle
        );
        assert_eq!(game.board.card_state(top).unwrap().status, CardStatus::Idle);
        assert!(game.history().is_empty());
    }

    #[test]
    fn unknown_target_square_blocks_an_intention() {
        let mut game = Game::new(Layout::standard(2).unwrap());
        let card = Card::from_display(Owner::Heart, 1, 2).unwrap();
        game.apply(Operation::Root(card)).unwrap();

        assert!(matches!(
            game.apply(Operation::Intention {
                card,
                target: Support::Base(Square::new(9, 9)),
            })
            .unwrap(),
            ApplyResult::Blocked(violations) if violations.contains(&Violation::UnknownTargetSquare(Square::new(9, 9)))
        ));
    }

    #[test]
    fn premature_end_is_a_reversible_blocked_batch() {
        let mut game = Game::new(Layout::standard(2).unwrap());
        let card = Card::from_display(Owner::Heart, 1, 2).unwrap();
        game.apply(Operation::Root(card)).unwrap();

        assert!(matches!(
            game.apply(Operation::End).unwrap(),
            ApplyResult::Blocked(violations) if violations == vec![Violation::PendingCards]
        ));
        assert!(game.undo());
        assert_eq!(game.history(), vec![HistoryEntry::Root(card)]);
    }
}
