//! Deterministic pile-based rules for Traffic in Hanoi.
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}
impl Direction {
    fn step(self, s: Square) -> Option<Square> {
        match self {
            Self::Up => s.row.checked_add(1).map(|r| Square::new(s.column, r)),
            Self::Down => s.row.checked_sub(1).map(|r| Square::new(s.column, r)),
            Self::Left => s.column.checked_sub(1).map(|c| Square::new(c, s.row)),
            Self::Right => s.column.checked_add(1).map(|c| Square::new(c, s.row)),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Adjacency {
    source: Square,
    direction: Direction,
}
impl Adjacency {
    pub fn new(source: Square, direction: Direction) -> Result<Self, CoreError> {
        direction
            .step(source)
            .ok_or_else(|| CoreError::new("adjacency leaves board coordinates"))?;
        Ok(Self { source, direction })
    }
    pub const fn source(self) -> Square {
        self.source
    }
    pub const fn direction(self) -> Direction {
        self.direction
    }
    pub fn target(self) -> Square {
        self.direction
            .step(self.source)
            .expect("validated adjacency")
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PlayerId(u16);
impl PlayerId {
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PlayerName(String);
impl PlayerName {
    pub fn new(value: impl Into<String>) -> Result<Self, CoreError> {
        let value = value.into();
        if value.trim().is_empty() {
            Err(CoreError::new("player name must not be empty"))
        } else {
            Ok(Self(value))
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Rank(u16);
impl Rank {
    pub fn new(value: u16) -> Result<Self, CoreError> {
        (value > 0)
            .then_some(Self(value))
            .ok_or_else(|| CoreError::new("rank must be positive"))
    }
    pub const fn value(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Card {
    pub owner: PlayerId,
    pub rank: Rank,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Pile(Vec<Card>);
impl Pile {
    pub fn new(cards: Vec<Card>) -> Result<Self, CoreError> {
        if cards.windows(2).any(|p| p[1].rank > p[0].rank) {
            Err(CoreError::new("pile is not rank-legal"))
        } else {
            Ok(Self(cards))
        }
    }
    pub fn empty() -> Self {
        Self(vec![])
    }
    pub fn cards(&self) -> &[Card] {
        &self.0
    }
    pub fn top(&self) -> Option<Card> {
        self.0.last().copied()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    fn pop(&mut self) -> Option<Card> {
        self.0.pop()
    }
    fn push(&mut self, c: Card) {
        self.0.push(c)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SquareState {
    Idling,
    Moving(Direction),
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SquareStatus {
    Planted,
    Evicted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlayerSetup {
    name: PlayerName,
    start: Square,
    finish: Square,
    initial_pile: Pile,
}
impl PlayerSetup {
    pub fn new(name: PlayerName, start: Square, finish: Square, initial_pile: Pile) -> Self {
        Self {
            name,
            start,
            finish,
            initial_pile,
        }
    }
    pub fn name(&self) -> &PlayerName {
        &self.name
    }
    pub const fn start(&self) -> Square {
        self.start
    }
    pub const fn finish(&self) -> Square {
        self.finish
    }
    pub fn initial_pile(&self) -> &Pile {
        &self.initial_pile
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GameDesign {
    players: Vec<PlayerSetup>,
    squares: BTreeSet<Square>,
}
impl GameDesign {
    pub fn new(players: Vec<PlayerSetup>, squares: BTreeSet<Square>) -> Result<Self, CoreError> {
        if !(2..=u16::MAX as usize).contains(&players.len()) {
            return Err(CoreError::new("a design needs two or more players"));
        }
        if squares.is_empty()
            || squares.iter().map(|s| s.column).min() != Some(0)
            || squares.iter().map(|s| s.row).min() != Some(0)
        {
            return Err(CoreError::new("squares must be non-empty and normalized"));
        }
        let mut names = BTreeSet::new();
        for (i, p) in players.iter().enumerate() {
            if !names.insert(p.name.clone()) {
                return Err(CoreError::new("player names must be unique"));
            }
            if !squares.contains(&p.start) || !squares.contains(&p.finish) {
                return Err(CoreError::new("special square missing from board"));
            }
            if p.initial_pile
                .cards()
                .iter()
                .any(|c| c.owner != PlayerId(i as u16))
            {
                return Err(CoreError::new("initial pile contains a foreign card"));
            }
        }
        for (i, p) in players.iter().enumerate() {
            for q in players.iter().skip(i + 1) {
                if [p.start, p.finish]
                    .into_iter()
                    .any(|s| s == q.start || s == q.finish)
                {
                    return Err(CoreError::new("different players' special squares overlap"));
                }
            }
        }
        Ok(Self { players, squares })
    }
    pub fn standard() -> Self {
        let squares = (0..=2)
            .flat_map(|x| {
                (0..=4).filter_map(move |y| {
                    (x != 1 || !matches!(y, 0 | 4)).then_some(Square::new(x, y))
                })
            })
            .collect();
        let pile = |o| {
            Pile::new(
                (1..=5)
                    .rev()
                    .map(|r| Card {
                        owner: o,
                        rank: Rank(r),
                    })
                    .collect(),
            )
            .expect("standard pile")
        };
        Self::new(
            vec![
                PlayerSetup::new(
                    PlayerName::new("Heart").unwrap(),
                    Square::new(0, 0),
                    Square::new(2, 4),
                    pile(PlayerId(0)),
                ),
                PlayerSetup::new(
                    PlayerName::new("Spade").unwrap(),
                    Square::new(0, 4),
                    Square::new(2, 0),
                    pile(PlayerId(1)),
                ),
            ],
            squares,
        )
        .unwrap()
    }
    pub fn players(&self) -> &[PlayerSetup] {
        &self.players
    }
    pub fn squares(&self) -> impl Iterator<Item = Square> + '_ {
        self.squares.iter().copied()
    }
    pub fn contains(&self, s: Square) -> bool {
        self.squares.contains(&s)
    }
    pub fn player(&self, p: PlayerId) -> &PlayerSetup {
        &self.players[p.index()]
    }
    pub fn player_id_at(&self, index: usize) -> Option<PlayerId> {
        (index < self.players.len()).then_some(PlayerId(index as u16))
    }
    fn next(&self, p: PlayerId) -> PlayerId {
        PlayerId(((p.index() + 1) % self.players.len()) as u16)
    }
    fn foreign_special(&self, o: PlayerId, s: Square) -> bool {
        self.players
            .iter()
            .enumerate()
            .any(|(i, p)| PlayerId(i as u16) != o && (p.start == s || p.finish == s))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PileHeight(u16);
impl PileHeight {
    pub const fn value(self) -> u16 {
        self.0
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CardLocation {
    square: Square,
    height: PileHeight,
}
impl CardLocation {
    pub const fn square(self) -> Square {
        self.square
    }
    pub const fn height(self) -> PileHeight {
        self.height
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct SquareContents {
    pile: Pile,
    state: SquareState,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Board {
    squares: BTreeMap<Square, SquareContents>,
}
impl Board {
    pub fn squares(&self) -> impl Iterator<Item = Square> + '_ {
        self.squares.keys().copied()
    }
    pub fn pile(&self, s: Square) -> &[Card] {
        &self.squares[&s].pile.0
    }
    pub fn state(&self, s: Square) -> SquareState {
        self.squares[&s].state
    }
    fn at(&self, s: Square) -> &SquareContents {
        &self.squares[&s]
    }
    fn at_mut(&mut self, s: Square) -> &mut SquareContents {
        self.squares.get_mut(&s).expect("design square")
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MatchSnapshot {
    board: Board,
    current_player: PlayerId,
    active_card: Option<CardLocation>,
}
impl MatchSnapshot {
    pub fn new(d: &GameDesign) -> Self {
        let mut squares = BTreeMap::new();
        for s in d.squares() {
            squares.insert(
                s,
                SquareContents {
                    pile: Pile::empty(),
                    state: SquareState::Idling,
                },
            );
        }
        for p in d.players() {
            squares.get_mut(&p.start).unwrap().pile = p.initial_pile.clone();
        }
        Self {
            board: Board { squares },
            current_player: PlayerId(0),
            active_card: None,
        }
    }
    pub fn board(&self) -> &Board {
        &self.board
    }
    pub const fn current_player(&self) -> PlayerId {
        self.current_player
    }
    pub const fn active_card(&self) -> Option<CardLocation> {
        self.active_card
    }
    pub fn status(&self, _: &GameDesign, s: Square) -> SquareStatus {
        let pile = &self.board.at(s).pile;
        let Some(top) = pile.top() else {
            return SquareStatus::Planted;
        };
        let owner = pile
            .cards()
            .iter()
            .take(pile.len().saturating_sub(1))
            .any(|c| c.owner != top.owner);
        let rank = self
            .board
            .squares()
            .any(|from| match self.board.state(from) {
                SquareState::Moving(dir) => {
                    dir.step(from) == Some(s)
                        && self
                            .board
                            .at(from)
                            .pile
                            .top()
                            .is_some_and(|m| top.rank < m.rank)
                }
                SquareState::Idling => false,
            });
        if owner || rank {
            SquareStatus::Evicted
        } else {
            SquareStatus::Planted
        }
    }
    pub fn is_settled(&self, d: &GameDesign) -> bool {
        self.board.squares().all(|s| {
            self.board.state(s) == SquareState::Idling && self.status(d, s) == SquareStatus::Planted
        })
    }
    pub fn winner(&self, d: &GameDesign) -> Option<PlayerId> {
        let p = self.current_player;
        self.board
            .squares()
            .all(|s| {
                self.board
                    .pile(s)
                    .iter()
                    .filter(|c| c.owner == p)
                    .all(|_| s == d.player(p).finish())
            })
            .then_some(p)
    }
    fn apply_action(&mut self, d: &GameDesign, e: &ServerEvent) -> Result<(), CoreError> {
        match *e {
            ServerEvent::Intend(a) => self.intend(d, a),
            ServerEvent::Fulfil(s) => self.fulfil(d, s),
            ServerEvent::EndTurn => self.end(d),
            ServerEvent::Undo => Err(CoreError::new("Undo requires replay history")),
        }
    }
    fn intend(&mut self, d: &GameDesign, a: Adjacency) -> Result<(), CoreError> {
        let source = a.source();
        let target = a
            .direction
            .step(source)
            .ok_or_else(|| CoreError::new("invalid direction"))?;
        if !d.contains(source) || !d.contains(target) {
            return Err(CoreError::new("adjacency leaves board"));
        }
        if self.board.state(source) != SquareState::Idling
            || self.board.state(target) != SquareState::Idling
        {
            return Err(CoreError::new("source and target must be idling"));
        }
        let card = self
            .board
            .at(source)
            .pile
            .top()
            .ok_or_else(|| CoreError::new("source pile is empty"))?;
        if d.foreign_special(card.owner, target) {
            return Err(CoreError::new("foreign special square"));
        }
        match self.active_card {
            None if card.owner == self.current_player => {
                self.active_card = Some(CardLocation {
                    square: source,
                    height: PileHeight((self.board.at(source).pile.len() - 1) as u16),
                })
            }
            None => {
                return Err(CoreError::new(
                    "initial intention must use current player's card",
                ));
            }
            Some(active)
                if self.status(d, source) == SquareStatus::Evicted
                    && ((active.square == source
                        && active.height.0 as usize == self.board.at(source).pile.len() - 1)
                        || card.owner != self.current_player) => {}
            Some(_) => {
                return Err(CoreError::new(
                    "subsequent intention requires evicted active or foreign card",
                ));
            }
        }
        self.board.at_mut(source).state = SquareState::Moving(a.direction);
        Ok(())
    }
    fn fulfil(&mut self, d: &GameDesign, source: Square) -> Result<(), CoreError> {
        if !d.contains(source) {
            return Err(CoreError::new("source is not on board"));
        }
        let direction = match self.board.state(source) {
            SquareState::Moving(x) => x,
            SquareState::Idling => return Err(CoreError::new("source is not moving")),
        };
        let target = direction
            .step(source)
            .ok_or_else(|| CoreError::new("invalid direction"))?;
        if !d.contains(target) {
            return Err(CoreError::new("target is not on board"));
        }
        let moving = self.board.at(source).pile.top().expect("moving has top");
        if self
            .board
            .at(target)
            .pile
            .top()
            .is_some_and(|top| top.rank < moving.rank)
        {
            return Err(CoreError::new("target rank is lower"));
        }
        let active = self.active_card.is_some_and(|a| {
            a.square == source && a.height.0 as usize == self.board.at(source).pile.len() - 1
        });
        let card = self.board.at_mut(source).pile.pop().unwrap();
        self.board.at_mut(source).state = SquareState::Idling;
        let height = PileHeight(self.board.at(target).pile.len() as u16);
        self.board.at_mut(target).pile.push(card);
        if active {
            self.active_card = Some(CardLocation {
                square: target,
                height,
            })
        }
        Ok(())
    }
    fn end(&mut self, d: &GameDesign) -> Result<(), CoreError> {
        if self.active_card.is_none() || !self.is_settled(d) {
            return Err(CoreError::new("turn is not settled"));
        }
        self.active_card = None;
        self.current_player = d.next(self.current_player);
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ServerEvent {
    Intend(Adjacency),
    Fulfil(Square),
    EndTurn,
    Undo,
}
pub fn replay(d: &GameDesign, log: &[ServerEvent]) -> Result<MatchSnapshot, CoreError> {
    let mut committed = MatchSnapshot::new(d);
    let mut current = committed.clone();
    let mut open = Vec::new();
    for e in log {
        match e {
            ServerEvent::Undo => {
                open.pop()
                    .ok_or_else(|| CoreError::new("nothing to undo"))?;
                current = committed.clone();
                for x in &open {
                    current.apply_action(d, x)?
                }
            }
            ServerEvent::EndTurn => {
                current.apply_action(d, e)?;
                committed = current.clone();
                open.clear()
            }
            _ => {
                current.apply_action(d, e)?;
                open.push(e.clone())
            }
        }
    }
    Ok(current)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoreError(String);
impl CoreError {
    pub fn new(m: impl Into<String>) -> Self {
        Self(m.into())
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
    fn initial_is_settled() {
        let d = GameDesign::standard();
        assert!(MatchSnapshot::new(&d).is_settled(&d))
    }
    #[test]
    fn replay_undo() {
        let d = GameDesign::standard();
        let l = vec![
            ServerEvent::Intend(Adjacency::new(Square::new(0, 0), Direction::Up).unwrap()),
            ServerEvent::Undo,
        ];
        assert!(replay(&d, &l).unwrap().is_settled(&d))
    }
}
