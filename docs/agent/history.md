# Planned client-local history

This is the approved replacement for the browser's temporary retained-log
replica. It is not implemented yet; see [implementation.md](implementation.md)
for the current replay-prefix bridge behavior.

Client review history is not the append-only server log. It is a reversible
view of surviving board actions, held beside a current authoritative snapshot.
The client does not store `EndTurn` or `Undo` entries in either history
collection.

```rust
struct ClientMatch {
    players: Vec<ClientPlayer>,
    past: Vec<BackwardRecord>,
    future: VecDeque<ForwardRecord>,
    viewed: MatchSnapshot,
    current: MatchSnapshot,
}

enum ForwardRecord {
    Intend(Adjacency),
    Fulfil(Square),
}

enum BackwardRecord {
    Intend(Square),
    Fulfil(Adjacency),
}
```

`players` retains names, ordered identities, and special squares. Bootstrap
uses the richer `GameDesign` once to construct the initial snapshot; the
client need not retain initial piles or an independent square set afterward.

## Stepping

Use **step backward** and **step forward** for local review. Do not call these
operations Undo: `Undo` is the distinct authoritative server event.

To step backward, pop from `past`, reverse it against `viewed`, and push its
forward form to the front of `future`. To step forward, pop the front of
`future`, apply it to `viewed`, and push its backward form to `past`.

The entries intentionally contain only the data unavailable in the snapshot
at the time that direction is applied:

- Before reversing `BackwardRecord::Intend(source)`, the source is moving, so
  its stored direction supplies the `Adjacency` required by the future entry.
- `BackwardRecord::Fulfil(adjacency)` retains the source and target because a
  fulfilled source is idling and no longer retains its direction.
- Before forwarding `ForwardRecord::Fulfil(source)`, the source's moving
  state supplies the adjacency required by the backward entry.

An adjacency always describes the original forward move from source to target.
Reverse fulfilment physically moves the target's top card back to the source;
it does not reverse the meaning of the stored adjacency.

## Inferred turn boundaries

An `EndTurn` is omitted from local history. The client recognizes a turn
boundary through the settled predicate: every square is idling and planted.
When crossing a settled boundary backward, it restores the preceding player
before reversing that turn's final fulfilment. When crossing it forward, the
final fulfilment is followed by the inferred player advance and clearing of
the active-card location.

Reversing an intention clears the active-card location exactly when the
resulting snapshot is settled. A subsequent intention cannot have that result,
because its source was evicted before it was intended.

The server's `Undo` is accepted only in an unended turn. Consequently the
client need not restore an active-card location across a committed `EndTurn`.
A future consensual take-back feature may reconstruct such a pre-end snapshot
from the client records, but is outside this version.
