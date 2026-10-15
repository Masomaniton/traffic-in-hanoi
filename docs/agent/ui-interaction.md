# Traffic in Hanoi: UI Interaction

The browser submits only source-oriented board actions. It never names a card
to move, calculates an eviction, or treats its local snapshot as authority.

## Live controls

- Select an idling non-empty source square and an orthogonal direction to
  submit `Intend(Adjacency)`.
- Select a moving source square to submit `Fulfil(source)`.
- Submit `EndTurn` only as an attempt; the server determines whether the
  current authoritative snapshot is settled.
- Submit `Undo` only for the open current turn. Once the server has accepted
  `EndTurn`, that turn is committed.

The UI may preview the adjacency and derived target, but an authoritative
server event is the only change to `current`.

## Status and accessibility

Render piles bottom-to-top. Render moving direction and derived square status
from the current/viewed snapshot. Empty squares are planted. The UI should
make the current player, active-card location, settled status, and any server
rejection understandable without relying only on colour.

## Local review

Use **step backward** and **step forward** for local history navigation. These
controls affect `viewed`, never `current`, and do not send `Undo` to the
server. The client may submit a gameplay event only while the viewed snapshot
is current.

When the user steps across a settled turn boundary, the client infers the
omitted `EndTurn` transition as defined in [history.md](history.md). A future
consensual take-back is not a local-review control.
