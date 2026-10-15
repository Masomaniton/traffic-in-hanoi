# Traffic in Hanoi: Implementation Specification

This document specifies the implementation shape for the pile-based rules in
[rules.md](rules.md). The human rules remain normative.

## 1. Core state

`traffic-core` uses the `GameDesign`, `Board`, and `MatchSnapshot` model in
[core-api.md](core-api.md). A board maps every board square to one pile and
one stored square state. It does not store card supports, per-card states,
eviction records, status flags, or a duplicate square index.

Piles are bottom-to-top and rank-legal. `SquareState::Moving` stores only a
direction. `Adjacency` stores a source and direction and derives its target;
all one-square coordinate arithmetic is centralized there.

Status is a pure query over a snapshot:

- rank eviction is caused by a moving square whose target has a lower-ranked
  top card;
- owner eviction is caused by a non-empty pile whose top-card owner differs
  from any card below it;
- otherwise the square is planted.

`MatchSnapshot` stores only the board, current player, and optional positional
active-card location. It contains no log, history cursor, status cache,
sequence number, or outcome cache.

## 2. Core event transitions

The replayable event type is:

```text
Intend(Adjacency) | Fulfil(Square) | EndTurn | Undo
```

`Intend` validates its source, derived target, square states, special-square
restriction, and initial/subsequent-turn condition before changing the source
to moving. An initial intention records the source top card's location as the
active-card location.

`Fulfil` validates that its source is moving and that the target is empty or
has a top rank no lower than the moving card. It pops the source top card,
pushes it onto the target pile, idles the source, and relocates the active-card
location only when that positional card moved.

`EndTurn` validates the settled predicate, clears the active-card location,
and advances the current player. The next player's victory condition is then
derived from the board.

For this version, `Undo` reverses only the final board action of the current
unended turn. It is rejected after `EndTurn`. Invalid events do not mutate the
snapshot and are not appended by the server.

## 3. Server and client containers

The authoritative room actor owns:

```text
ServerMatch { design, log, current }
```

It checks authenticated authority and stale network sequence before applying a
prospective core event to `current`. On success, it appends that event to
`log`, persists it, and broadcasts it. `current` is required for legal-move
validation and must never be supplied by a client.

The planned browser review model is:

```text
ClientMatch { players, past, future, viewed, current }
```

`current` is the latest authoritative snapshot; `viewed` is the locally
reviewed snapshot. The `past` vector and `future` deque are local navigation
state, not a second canonical log. Their compact, direction-specific records
are specified in [history.md](history.md).

The current web implementation has not yet added this review cursor. It
retains the received ordered event vector beside its latest reconstructed
snapshot. That bridge supports bootstrap and contiguous event validation, but
it is not the final client storage model. The target board geometry, pile
rendering, pointer interactions, and keyboard behavior are specified in
[ui.md](ui.md).

## 4. Networking and reconstruction

`traffic-network` gives vector positions their wire-visible `LogSequence`
values. The core does not know sequences. On a cold bootstrap, construct the
initial snapshot from `GameDesign` and replay the complete ordered event log.
On catch-up, require contiguous events after the retained network sequence.

The server's in-memory `current` snapshot is a cache of this deterministic
reconstruction. The durable source remains the immutable design and
append-only event vector.

## 5. Future take-backs

Undo is intentionally not a cross-turn correction feature. A future take-back
must be a separate request-and-consent protocol, capable of reconstructing an
earlier committed position only after the affected players approve it. It must
not be represented as a permissive extension of `Undo`.
