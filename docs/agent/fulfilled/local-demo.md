# Local pile-based demo

The local demo is an in-memory two-player implementation of the pile-based
rules. It has private rooms, anonymous cookie sessions, WebSockets, an
authoritative room-owned MatchSnapshot, and an append-only event vector.
Restarting the server discards every room and match.

## Prerequisites

Install the browser target and Trunk once:

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk
```

## Build and run

Build the browser bundle into the server's static directory, then run the
server from the repository root:

```sh
cd crates/traffic-web
trunk build --release --dist ../traffic-server/static
cd ../..
cargo run -p traffic-server
```

If a shell exports NO_COLOR=1, Trunk 0.21 interprets it incompatibly. Use
this equivalent build command instead:

```sh
env -u NO_COLOR trunk build --release --dist ../traffic-server/static
```

Open http://127.0.0.1:3000 in two distinct browser profiles. Distinct
profiles are necessary because the room seats are assigned to anonymous
cookie sessions.

## Play

1. In the first profile, select **Create room** and copy the displayed code.
2. In the second profile, enter that code and select **Join room**.
3. In each profile, enter the code and select **Connect**.
4. Click a non-empty idling source square, then an orthogonally adjacent target
   square, to submit an Intend. Click a moving square to submit its Fulfil.
   Use **End turn** only after the board is settled. **Undo** reverses the last
   action of the current unended turn.
5. Press and hold a non-empty pile to fan it out; the next click dismisses the
   fan without taking a board action. Left/right arrows review one logged
   event; up/down arrows review turn boundaries. Gameplay is disabled while
   reviewing an earlier position.

The server validates the acting player, known log sequence, and rules against
its authoritative snapshot before broadcasting any event. Invalid commands
show a rejection and do not change the shared state.

The browser currently requests a replay bootstrap on connection and retains
the received event log to rebuild its local snapshot. It provides read-only
local review by replaying a selected log prefix. The protocol supports
catch-up, but automatic reconnect/catch-up and the planned compact local
history representation remain follow-up work.
