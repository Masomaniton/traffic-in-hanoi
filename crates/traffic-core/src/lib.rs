use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

mod board;
mod history;
mod rules;
mod violation;
pub use board::{Board, CardState, CardStatus};
pub use history::HistoryCursor;
pub use violation::Violation;

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
pub enum Operation {
    Root(Card),
    Intention { card: Card, target: Support },
    Fulfilment(Card),
    End,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum HistoryEntry {
    Root(Card),
    Intention(Card),
    Fulfilment { source: Support, card: Card },
    End,
    Eviction(Card),
}

/// The forward representation of a local history entry. Unlike `HistoryEntry`,
/// it retains command fields needed to apply the next state transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ForwardHistoryEntry {
    Root(Card),
    Intention { card: Card, target: Support },
    Fulfilment(Card),
    End,
    Eviction(Card),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum GameDelta {
    Operation(Operation),
    Undo,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct GameSequence(u64);

impl GameSequence {
    pub const INITIAL: Self = Self(0);

    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GameCreated {
    pub(crate) layout: Layout,
}

impl GameCreated {
    pub const fn new(layout: Layout) -> Self {
        Self { layout }
    }

    pub const fn layout(&self) -> &Layout {
        &self.layout
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GameEvent {
    sequence: GameSequence,
    delta: GameDelta,
}

impl GameEvent {
    pub const fn new(sequence: GameSequence, delta: GameDelta) -> Self {
        Self { sequence, delta }
    }

    pub const fn sequence(&self) -> GameSequence {
        self.sequence
    }

    pub const fn delta(&self) -> &GameDelta {
        &self.delta
    }
}

impl fmt::Display for HistoryEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Root(card) => write!(f, "Root({card})"),
            Self::Intention(card) => write!(f, "Intention({card})"),
            Self::Fulfilment { source, card } => write!(f, "Fulfilment({card} from {source})"),
            Self::End => f.write_str("End"),
            Self::Eviction(card) => write!(f, "Eviction({card})"),
        }
    }
}

mod game;
pub use game::{ApplyResult, Game, GameState, TurnRecord};
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
mod tests;
