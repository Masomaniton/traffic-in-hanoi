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
4. Use an arrow beside a source square to submit an Intend; use **Fulfil** on a
   moving square; use **End turn** only after the board is settled. **Undo**
   reverses the last action of the current unended turn.

The server validates the acting player, known log sequence, and rules against
its authoritative snapshot before broadcasting any event. Invalid commands
show a rejection and do not change the shared state.

The browser currently requests a replay bootstrap on connection and retains
the received event log to rebuild its local snapshot. The protocol supports
catch-up, but automatic reconnect/catch-up and client-local review controls
remain follow-up work.
