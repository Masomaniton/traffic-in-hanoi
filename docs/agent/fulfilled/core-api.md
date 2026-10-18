# `traffic-core` API

`traffic-core` is the deterministic Rust rules engine. It owns game-domain
values, configuration validation, snapshot transitions, and replayable server
events. It owns no sockets, persistence, sequence numbers, authentication,
clock, or UI behavior.

## Core model

The core distinguishes immutable match configuration from a point-in-time
snapshot. A snapshot contains no history and no transport metadata: it has
only what is required to continue the match.

```rust
struct Board {
    squares: BTreeMap<Square, SquareContents>,
}

struct GameDesign {
    players: Vec<PlayerSetup>,
    squares: BTreeSet<Square>,
}

struct PlayerSetup {
    name: PlayerName,
    start: Square,
    finish: Square,
    initial_pile: Pile,
}

struct MatchSnapshot {
    board: Board,
    current_player: PlayerId,
    active_card: Option<CardLocation>,
}

struct CardLocation {
    square: Square,
    height: PileHeight,
}
```

`Board` has one entry for every configured square, including empty squares.
The board is constructed from a validated `GameDesign`; its private mutation
API never adds or removes squares. `GameDesign` therefore describes the board
for configuration and bootstrap, while the board itself is the sole square
map in a snapshot.

The index of a player in `GameDesign::players` is its `PlayerId`. The newtype
exists for type safety, not as a duplicated identifier. A player setup owns no
explicit ID field.

Piles are bottom-to-top sequences of cards. A `Pile` constructor validates the
rank-legal invariant. Square state is stored as `Idling` or
`Moving(Direction)`. Square status is never stored: `Planted` and `Evicted`
are derived from the current piles and moving-square states.

## Adjacencies and events

An adjacency stores the source and direction of one orthogonal step. Its
target is derived by its getter, centralizing coordinate arithmetic.

```rust
struct Adjacency {
    source: Square,
    direction: Direction,
}

impl Adjacency {
    fn source(&self) -> Square;
    fn target(&self) -> Square;
}

enum ServerEvent {
    Intend(Adjacency),
    Fulfil(Square),
    EndTurn,
    Undo,
}
```

`ServerEvent` is a core type because replaying it is a deterministic rules
operation. The authoritative server alone decides whether to append one. The
network crate assigns and carries log sequence numbers around these values.

`Intend` identifies only a source/target adjacency; its moving card is the
source pile's top card. `Fulfil` identifies only the moving source square; its
target is the direction stored in that square. Clients never name cards to
move.

## Turn and completion rules

`active_card == None` represents an initial turn. A successful initial
intention establishes `Some(CardLocation)`; that location moves only if that
card is fulfilled. A subsequent turn is therefore represented without a
separate phase field.

A snapshot is **settled** exactly when every square is idling and planted.
`EndTurn` is valid only from a settled snapshot. It clears the active-card
location and advances the current player in configured order. Winning is
derived at the start of a player's turn from the board and player setup; it is
not a stored snapshot field.

For this version, `Undo` is limited to an unended turn. Once `EndTurn` is
accepted, the turn is committed and a later `Undo` is rejected. A future,
consensual take-back mechanism is distinct from `Undo`.

## Validation and replay

The core validates a prospective event against a `GameDesign` and mutable
`MatchSnapshot`. Failed attempts leave the snapshot unchanged; the server does
not append rejected events. Applying the same valid design and ordered event
log always produces the same snapshot.

The core deliberately does not define a server or browser container. Those
containers, their histories, and all sequence handling are described in
[history.md](history.md) and [network.md](network.md).
