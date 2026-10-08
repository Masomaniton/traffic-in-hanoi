# `traffic-network`

`traffic-network` contains only serializable, transport-neutral messages shared
by the server and browser. It depends on `traffic-core` for replay values, but
does not contain WebSocket code, HTTP code, authentication, or game rules.

## Canonical log

The server persistently owns one append-only sequence of `GameEvent` values per
game. `GameCreated` is separate immutable setup data. Every accepted delta,
including `Undo` and a delta that creates a blocked batch, becomes exactly one
event. Derived evictions do not become network events because replay derives
them deterministically.

## Synchronization modes

**Catch-up** is memory-preserving reconnection. The client retains a locally
replayed game state and asks for the events after its highest contiguous
`GameSequence`.

**Replay bootstrap** is memoryless re-entry. An authenticated account may open
the game with no local state; the server supplies `GameCreated` and the full
event log. The browser reconstructs state locally without a server snapshot.

## Submission

`SubmitDelta` includes the sequence the client had applied when it chose the
delta. The server appends it only if that is its current sequence. A duplicate
retry is then stale after the original was accepted, so it cannot append a
second operation or create an accidental blocked batch.
