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

pub enum CardStatus {
    Idle,
    Evicted,
    Intended(Support),
}

pub struct Board {
    pub cards: BTreeMap<Card, CardState>,
}
```

`Owner` is preferable to `Suit`: Heart and Spade drive ownership mechanics.
`Rank` stores its internal zero-based value and displays as `value + 1`.

`CardState` intentionally stores only direct support. `square_of(card)` follows
support links down to `Base(square)`. `occupants(support)` scans the card table
and permits more than one direct occupant while a blocked fulfilment is being
reviewed; `is_occupied(support)` is the corresponding existence query. These
operations are bounded by the configurable card count, avoid divergent indexes,
and should be optimized only after profiling.

`CardStatus::Idle` is a card at rest, `Evicted` is a card temporarily lifted
from its support, and `Intended(target)` records an announced but unfulfilled
move. The target is repeated in this status because it is current game state,
not history; it is required to validate and fulfil the intention.

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

The shared `traffic-core` engine maintains all surviving history as ordered,
reversible entries:

```text
Root(card)
| Intention(card)
| Fulfilment { source, card }
| End
| Eviction(card)
```

`Root`, `Intention`, `Fulfilment`, and `End` are player operations. `Eviction`
is automatic.

`Fulfilment` records the moving card and its source support, but not a target.
While unapplying, the card's current support is the target. Its inverse
restores direct support to `source` and status to `Intended(target)`. A card's
square is derived from direct supports and is never stored independently.

The entry types are direct, symmetric mutations. A blocked attempted operation
still contributes its operation entry (and any preceding automatic evictions)
so Undo can remove it. No board snapshots or per-event undo journal are kept.

A **batch** starts with one operation and contains its following zero or more
derived eviction entries. Undo removes the final batch. This grouping permits
automatic evictions to be individually replayed while retaining correct causal
rollback. Local left/right history navigation reverses or reapplies the whole
batch: it visits trailing evictions first, then its player operation.

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
is blocked or valid. A valid `End` flips the active player and is retained as a
history entry; completed turn records are derived from surviving entries. An
invalid `End` is merely a blocked final batch.

## 5. Networking and replay

The canonical network and persistence record is an append-only sequence of
accepted game deltas. It is deliberately thinner than local reversible
history: a delta expresses the attempted player command, while `traffic-core`
derives source supports, resolved fulfilment transitions, evictions, and
violations. A delta is an operation or the Undo control:

```text
GameDelta = Operation(
  Root(card)
  | Intention { card, target }
  | Fulfilment(card)
  | End
) | Undo
```

`GameCreated` is immutable setup data. It provides the layout and rank count.
The sequenced `GameEvent` log and stale-submission contract are specified in
[network.md](network.md). The acting owner is derivable from replay state
before each event; authentication and audit metadata belong to the server, not
the core replay record.
Automatic evictions are derived by `traffic-core`; clients never submit or
receive them as separate network deltas. In particular, the network
`Fulfilment(card)` retains the player's attempted card identity; the local
history resolves it into `Fulfilment { source, card }` without duplicating the
target.

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

The directional cursor, batch navigation, and inverse entry types are
specified in [history.md](history.md).
