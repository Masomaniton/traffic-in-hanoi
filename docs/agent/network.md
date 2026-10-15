# `traffic-network`

`traffic-network` contains serializable transport-neutral messages shared by
the server and browser. It depends on `traffic-core` for `GameDesign`,
`MatchSnapshot`, and `ServerEvent`; it contains no game rules, sockets, HTTP,
or authentication.

## Sequenced server log

The authoritative server stores one append-only `Vec<ServerEvent>` per match.
An event's zero-based index in that vector is its log sequence. Sequence
numbers are network metadata, not core state and not fields of
`MatchSnapshot`.

```rust
pub struct LogSequence(u64);

pub struct SequencedEvent {
    pub sequence: LogSequence,
    pub event: ServerEvent,
}
```

The server uses the vector position when persisting or reconstructing a match.
`SequencedEvent` is used at the transport boundary, where a recipient must
identify gaps, duplicates, and catch-up position.

Each accepted `ServerEvent` is appended exactly once. Invalid commands are
rejected and never enter the canonical log. `Undo` is logged even though its
effect is to remove the final open-turn board action from the live snapshot.
`EndTurn` is logged even though it is omitted from client review history.

## Synchronization

`SubmitEvent` includes the client's highest contiguous `LogSequence` and a
prospective `ServerEvent`. The actor rejects a stale submission before rules
validation. This prevents retried or concurrent actions from being evaluated
against a newer board.

**Catch-up** retains client memory. A client asks for events strictly after its
known sequence and applies them in contiguous order.

**Replay bootstrap** is memoryless re-entry. The server sends `GameDesign` and
the complete ordered event log. The browser constructs its initial snapshot
from the design, replays the log, then retains only its `ClientPlayer` data,
`ClientMatch` review records, and live snapshots.

No client-facing mutable-state snapshot is transmitted as an authority. A
network wrapper may pair a received sequence with a `MatchSnapshot` for local
bookkeeping, but the snapshot itself remains sequence-free.
