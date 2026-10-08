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

## 5. Networking

Client commands are operations or the Undo control:

```text
Root(card) | Intention(card, support) | Fulfilment(card, support) | End | Undo
```

The server validates commands authoritatively and broadcasts ordered updates
containing accepted entries, batch status, and validation reasons. Clients do
not submit evictions directly.

Updates are provisional while a turn is open. Reconnect uses a turn-start
snapshot plus the current ordered history, which deterministically rebuilds
batches and blocked status.

