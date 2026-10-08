# Local two-player demo

This is an intentionally bare-bones, in-memory demo. It has private rooms,
anonymous cookie sessions, WebSockets, a replayed event log, and controls for
every game delta. Restarting the server discards every room and game.

## Prerequisites

Install the browser target and Trunk once:

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk
```

## Build and run

From the repository root, build the browser bundle into the directory served by
the Axum server:

```sh
cd crates/traffic-web
trunk build --release --dist ../traffic-server/static
cd ../..
cargo run -p traffic-server
```

Open `http://127.0.0.1:3000` in two distinct Firefox profiles. Distinct
profiles are important: they maintain separate anonymous session cookies while
using the same local origin.

## Play a local game

1. In Heart's profile, click **Create room (Heart)**. Copy the displayed room
   code, then click **Connect**.
2. In Spade's profile, paste the room code, click **Join room (Spade)**, then
   click **Connect**. **Connect alone does not claim the Spade seat.**
3. Both profiles receive the same replay bootstrap and show the live board.
4. Use a card value such as `H1` and the controls to root, intend, fulfil, end,
   or undo. An intention target is either `base x y` (for example `base 0 1`)
   or `card H1`.

The server broadcasts accepted deltas to both profiles. The browser replays
them through `traffic-core`; automatic evictions and blocked batches are never
sent as independent network commands.

Heart must finish the entire turn with **End** before Spade can root `S1`.
The board's `active` line identifies whose turn it is.

## Re-entry modes

Refreshing a connected profile performs **replay bootstrap**: the server sends
`GameCreated` plus the complete event log. Future UI work will use
**catch-up** for a connection drop when the browser still retains its local
replica.
