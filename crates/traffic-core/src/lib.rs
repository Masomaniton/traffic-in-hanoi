use std::collections::{BTreeMap, BTreeSet, VecDeque};
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
    pub fn occupants(&self, support: Support) -> impl Iterator<Item = Card> + '_ {
        self.cards
            .iter()
            .filter_map(move |(card, state)| (state.support == support).then_some(*card))
    }
    pub fn is_occupied(&self, support: Support) -> bool {
        self.occupants(support).next().is_some()
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
    layout: Layout,
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
pub struct TurnRecord {
    pub player: Owner,
    pub entries: Vec<HistoryEntry>,
}
/// Client-local position within the surviving reversible history. The
/// append-only `GameEvent` log is deliberately not represented here.
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
#[derive(Clone, Debug)]
pub enum ApplyResult {
    Applied,
    Blocked(Vec<Violation>),
    Ended(Owner),
}

#[derive(Clone, Debug)]
pub struct Game {
    created: GameCreated,
    layout: Layout,
    sequence: GameSequence,
    events: Vec<GameEvent>,
    board: Board,
    active_player: Owner,
    active_card: Option<Card>,
    past: Vec<HistoryEntry>,
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

    fn apply_forward_history_entry(&mut self, entry: ForwardHistoryEntry) -> HistoryEntry {
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

    fn unapply_history_entry(&mut self, entry: HistoryEntry) -> ForwardHistoryEntry {
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

    fn apply_operation_entry(&mut self, operation: Operation) -> Result<(), CoreError> {
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
                self.board
                    .cards
                    .get_mut(&card)
                    .ok_or_else(|| CoreError::new(format!("unknown card {card}")))?
                    .support = target;
                self.board
                    .cards
                    .get_mut(&card)
                    .ok_or_else(|| CoreError::new(format!("unknown card {card}")))?
                    .status = CardStatus::Idle;
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

    fn validate_intention_position(&self, card: Card) -> Result<Vec<Violation>, CoreError> {
        let state = self
            .board
            .cards
            .get(&card)
            .ok_or_else(|| CoreError::new(format!("unknown card {card}")))?;
        let mut v = Vec::new();
        let CardStatus::Intended(target) = state.status else {
            v.push(Violation::NotEvicted(card));
            return Ok(v);
        };
        if self.board.is_occupied(Support::Card(card)) {
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
        if let Some(occupant) = self
            .board
            .occupants(target)
            .find(|occupant| *occupant != card)
            && occupant.rank >= card.rank
        {
            v.push(Violation::CannotPushEqualOrLarger);
        }
        for (other, state) in self.board.cards() {
            if other == card {
                continue;
            }
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
    fn fulfilment_violations(&self, card: Card) -> Vec<Violation> {
        let Some(state) = self.board.cards.get(&card) else {
            return vec![Violation::NotIntended(card)];
        };
        let mut v = Vec::new();
        if self.board.is_occupied(Support::Card(card)) {
            v.push(Violation::CoveredMover(card));
        }
        if self
            .board
            .occupants(state.support)
            .any(|occupant| occupant != card)
        {
            v.push(Violation::OccupiedTarget(state.support));
        }
        v
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
        assert!(matches!(game.undo().unwrap(), ApplyResult::Applied));
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

        assert!(matches!(game.undo().unwrap(), ApplyResult::Applied));
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
        assert!(matches!(game.undo().unwrap(), ApplyResult::Applied));
        assert_eq!(game.history(), vec![HistoryEntry::Root(card)]);
    }

    #[test]
    fn events_replay_to_the_same_state_including_undo() {
        let created = GameCreated::new(Layout::standard(2).unwrap());
        let card = Card::from_display(Owner::Heart, 1, 2).unwrap();
        let mut original = Game::from_created(created.clone());
        original
            .apply_delta(GameDelta::Operation(Operation::Root(card)))
            .unwrap();
        original.apply_delta(GameDelta::Undo).unwrap();

        let events = original.events().to_vec();
        let mut replay = Game::from_created(created);
        for event in events {
            replay.apply_event(event).unwrap();
        }

        assert_eq!(original.sequence(), replay.sequence());
        assert_eq!(original.board, replay.board);
        assert_eq!(original.active_player(), replay.active_player());
        assert_eq!(original.active_card(), replay.active_card());
        assert_eq!(original.history(), replay.history());
    }

    #[test]
    fn fulfilment_history_records_the_card_and_source_support() {
        let mut game = Game::new(Layout::standard(2).unwrap());
        let card = Card::from_display(Owner::Heart, 1, 2).unwrap();
        let lower = Card::from_display(Owner::Heart, 2, 2).unwrap();
        let target = Support::Base(Square::new(0, 1));

        game.apply(Operation::Root(card)).unwrap();
        game.apply(Operation::Intention { card, target }).unwrap();
        game.apply(Operation::Fulfilment(card)).unwrap();

        assert!(game.history().contains(&HistoryEntry::Fulfilment {
            source: Support::Card(lower),
            card,
        }));
    }

    #[test]
    fn cursor_steps_a_complete_history_batch_without_replay() {
        let mut game = Game::new(Layout::standard(2).unwrap());
        let card = Card::from_display(Owner::Heart, 1, 2).unwrap();
        game.apply(Operation::Root(card)).unwrap();
        let mut cursor = HistoryCursor::from_game(game);

        assert!(cursor.step_back_batch());
        assert_eq!(
            cursor.game().board().card_state(card).unwrap().status,
            CardStatus::Idle
        );
        assert!(cursor.step_forward_batch());
        assert_eq!(
            cursor.game().board().card_state(card).unwrap().status,
            CardStatus::Evicted
        );
    }

    #[test]
    fn cursor_ingests_undo_while_reviewing_without_losing_its_position() {
        let created = GameCreated::new(Layout::standard(2).unwrap());
        let card = Card::from_display(Owner::Heart, 1, 2).unwrap();
        let mut game = Game::from_created(created);
        game.apply_delta(GameDelta::Operation(Operation::Root(card)))
            .unwrap();
        let (_, undo_outcome) = game.apply_delta(GameDelta::Undo).unwrap();
        assert!(matches!(undo_outcome, ApplyResult::Applied));

        let mut reviewed = Game::from_created(game.created().clone());
        reviewed.apply_event(game.events()[0].clone()).unwrap();
        let mut cursor = HistoryCursor::from_game(reviewed);
        assert!(cursor.step_back_batch());

        assert!(cursor.apply_event(game.events()[1].clone()));
        assert!(cursor.is_live());
        assert_eq!(
            cursor.game().board().card_state(card).unwrap().status,
            CardStatus::Idle
        );
    }

    #[test]
    fn cursor_rejects_a_gap_transactionally() {
        let mut cursor = HistoryCursor::from_game(Game::new(Layout::standard(2).unwrap()));
        let card = Card::from_display(Owner::Heart, 1, 2).unwrap();
        assert!(!cursor.apply_event(GameEvent::new(
            GameSequence(2),
            GameDelta::Operation(Operation::Root(card)),
        )));
        assert!(cursor.game().history().is_empty());
        assert!(cursor.is_live());
    }

    #[test]
    fn cursor_stays_live_when_an_event_arrives_live() {
        let mut source = Game::new(Layout::standard(2).unwrap());
        let card = Card::from_display(Owner::Heart, 1, 2).unwrap();
        let (event, _) = source
            .apply_delta(GameDelta::Operation(Operation::Root(card)))
            .unwrap();
        let mut cursor = HistoryCursor::from_game(Game::new(Layout::standard(2).unwrap()));
        assert!(cursor.apply_event(event));
        assert!(cursor.is_live());
        assert_eq!(
            cursor.game().board().card_state(card).unwrap().status,
            CardStatus::Evicted
        );
    }
}
