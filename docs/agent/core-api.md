# `traffic-core` API

`traffic-core` is the deterministic Rust rules engine. It owns no networking,
database, clock, randomness, or UI behavior.

## Source layout

`lib.rs` is the public facade for domain and event values. Implementation
responsibilities are separated internally:

- `board.rs` owns direct-support board state and derived board queries.
- `game.rs` owns replay state, event sequencing, Undo, and derived turn
  records.
- `rules.rs` owns deterministic entry mutations, incremental validation,
  automatic evictions, and completion checks.
- `history.rs` owns the client-local `HistoryCursor`.
- `violation.rs` owns blocked-operation violation values.
- `tests.rs` contains the table-driven core regression suite.

These are implementation boundaries, not separate crates or wire contracts.

## Current public model

The engine exposes `Game` (also available as the replay-oriented `GameState`
alias), whose fields are private, and public value types used to build layouts
and submit operations:

```rust
Game::new(layout)
game.apply(operation)
game.undo()
```

Browser presentation state uses the opaque `HistoryCursor`, constructed from a
fully replayed `Game`. It exposes `game()`, `is_live()`, directional
`step_back_batch()` / `step_forward_batch()` methods, and transactional
`apply_event(event) -> bool` ingestion. The cursor's surviving local history
is separate from the append-only event log; Undo is an event, not a cursor
entry.

`Layout` supports any finite, normalized set of non-negative coordinate squares
with orthogonal adjacency. It contains a configurable rank count and four
distinct special squares. `Card` uses `Owner` (`Heart` or `Spade`) and a
validated zero-based `Rank`; display ranks are one-based.

The board stores only each card's direct `Support`:

```rust
Support::Base(Square)
Support::Card(Card)
```

The engine derives a card's square by following its support chain. This avoids
maintaining duplicate square and support indexes.

## Operations and outcomes

The four rule operations are:

```rust
Operation::Root(Card)
Operation::Intention { card: Card, target: Support }
Operation::Fulfilment(Card)
Operation::End
```

`Game::apply` returns `ApplyResult::Applied`, `Blocked(violations)`, or
`Ended(owner)`. Blocked operations are deliberately retained as the final
reversible batch; `undo` restores the state before that entire batch.

## Replay API

`GameCreated` is immutable, serializable setup data. `GameDelta` is either one
of the four operations or `Undo`. Applying a delta to a local game produces a
serializable `GameEvent` with the next monotonically increasing
`GameSequence`. `apply_event` applies a received event only when its sequence
is exactly next, making gaps and duplicates explicit errors.

Both the server and browser reconstruct the same opaque state from
`GameCreated` and an ordered log of accepted deltas. See
[implementation.md](implementation.md) and [network.md](network.md) for the
transport and replay contract.

## Tests

Core tests live in `src/tests.rs`. Rules changes must cover valid and invalid
intermediate positions, deterministic replay, Undo, directional cursor steps,
and transactional event ingestion.
