# Traffic in Hanoi: Implementation Specification

This document specifies the online implementation of the declarative rules in
[rules.md](rules.md).

## 1. Rust domain model

Use a normalized orthogonal layout; do not support arbitrary graph edges in the
first version.

```rust
pub struct Square {
    pub column: u16,
    pub row: u16,
}

pub enum Owner {
    Heart,
    Spade,
}

pub struct Rank(u16); // validated: 0 <= rank < layout.rank_count

pub struct Card {
    pub owner: Owner,
    pub rank: Rank,
}

pub enum Support {
    Base(Square),
    Card(Card),
}

pub struct CardState {
    pub support: Support,
    pub status: CardStatus,
}

pub struct Board {
    pub cards: BTreeMap<Card, CardState>,
}
```

`Owner` is preferable to `Suit`: Heart and Spade drive ownership mechanics.
`Rank` stores its internal zero-based value and displays as `value + 1`.

`CardState` intentionally stores only direct support. `square_of(card)` follows
support links down to `Base(square)`; `occupant(support)` scans the card table.
Both are bounded by the configurable card count, avoid divergent indexes, and
should be optimized only after profiling.

```rust
pub struct Layout {
    pub rank_count: u16,
    pub squares: BTreeSet<Square>,
    pub starts: BTreeMap<Owner, Square>,
    pub finishes: BTreeMap<Owner, Square>,
}
```

`Layout` validation checks unique normalized coordinates, the presence of every
special square, and valid rank count. Adjacency is derived from coordinate
Manhattan distance one.

## 2. History and batches

The shared `traffic-core` engine maintains the current turn as ordered history
entries:

```text
Root(card) | Intention(card, support) | Fulfilment(card, support) | End | Eviction(card)
```

`Root`, `Intention`, `Fulfilment`, and `End` are player operations. `Eviction`
is automatic.

A **batch** starts with one operation and contains its following zero or more
derived eviction entries. Undo removes the final batch. This grouping permits
automatic evictions to be individually replayed while retaining correct causal
rollback.

## 3. Incremental validation and blocked batches

Append an operation entry and validate it immediately. If it is valid, append
each derived eviction one at a time, validating immediately after each entry.

Stop at the first invalid history position. Collect every violated predicate at
that position, but append no later eviction. The final batch is then **blocked**
and accepts no later operation until Undo removes it.

Validation is local and incremental; it neither globally scans the whole board
nor searches whether an eviction sequence is ultimately solvable.

## 4. Undo and turn records

Nothing in an open turn is permanent. Undo removes the final batch, whether it
is blocked or valid. A valid `End` seals the turn into an immutable turn record
for match replay; an invalid `End` is merely a blocked final batch.

## 5. Networking and replay

The canonical network and persistence record is an append-only sequence of
accepted game deltas. A delta is an operation or the Undo control:

```text
GameDelta = Operation(Root | Intention | Fulfilment | End) | Undo
```

`GameCreated` is immutable sequence-zero setup data. It provides the layout and
rank count. Each later `GameEvent` has a monotonically increasing unsigned
`GameSequence` and one accepted `GameDelta`. The acting owner is derivable from
the replay state before the event; authentication and audit metadata belong to
the server, not the core replay record.
Automatic evictions are derived by `traffic-core`; clients never submit or
receive them as separate network deltas.

The server validates commands authoritatively and broadcasts accepted events in
sequence order. Rejected transport commands—such as unauthorized commands or
commands based on a stale sequence—are not game events. A blocked attempt is a
valid game event because it adds a blocked batch, and an Undo is a valid game
event because it removes one.

Every client retains the complete local event history. A fresh client receives
sequence zero and all later events; a reconnecting client requests events after
its highest contiguous sequence. Replaying from `GameCreated` reconstructs
the same complete state, including every automatic eviction, blocked batch, and
undo. Server-side checkpoints may be added later solely as a replay-speed
optimization; clients need not receive snapshots.

The server uses an actor to serialize all commands for a game. A client submits
the sequence it has applied along with a prospective delta. This is not an
attempt to compensate for TCP ordering: WebSockets already provide ordered,
reliable delivery on an active connection. It prevents actions made against a
stale state from being interpreted against a newer board after reconnects,
retries, or concurrent player activity.

## 6. Local history navigation

`HistoryCursor` is client-only presentation state and is never transmitted. It
points to a **batch boundary**, not an individual history entry. Left/right
navigation therefore moves before or after one player operation together with
all its derived evictions—the same granularity as Undo. Up/down navigation
moves between turn boundaries. Gameplay input is available only at the current
live boundary.
