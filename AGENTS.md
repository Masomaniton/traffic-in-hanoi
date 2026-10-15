# Traffic in Hanoi — Agent Guide

## Product intent

Build a browser-playable, real-time implementation of **Traffic in Hanoi**. Players create or join private rooms, play a complete game together, reconnect safely after a temporary disconnect, and can review the completed game.

The application is a **100% Rust stack**: Rust on the server and Rust compiled to WebAssembly in the browser. Do not introduce JavaScript or TypeScript application code. Small declarative JavaScript-free HTML and CSS assets are fine.

## Documentation ownership

`docs/human/` contains human-authored notes. `docs/agent/` contains the
technical specifications and implementation documentation maintained during
agent work. Keep the authorship distinction out of the documents themselves.

## Workspace ownership

| Crate | Owns | Must not own |
| --- | --- | --- |
| `traffic-core` | Rules, cards, legal actions, game state, deterministic transitions, scoring, serialization types | HTTP, WebSockets, database access, UI framework types, clocks or randomness supplied implicitly |
| `traffic-network` | Transport-neutral replay and WebSocket message types shared by server and web | Rules, sockets, HTTP, authentication, database access, UI rendering |
| `traffic-server` | Authentication/session handling, room lifecycle, WebSocket protocol, persistence adapters, authorization, server-side orchestration | Duplicated rules or UI rendering |
| `traffic-web` | Leptos UI, client state, WebSocket client, accessibility, responsive styling | Independent game-rule implementation or direct database access |

When a new responsibility does not clearly fit, prefer a new narrowly focused crate (for example `traffic-protocol` for shared wire messages) over weakening these boundaries.

## Recommended architecture

```text
Browser (traffic-web, Leptos + WASM)
  └─ traffic-network WebSocket messages / local replay state
Server (traffic-server, Axum + Tokio)
  ├─ room actor per active game
  ├─ authentication and authorization
  ├─ PostgreSQL persistence
  └─ calls deterministic transitions
Rules engine (traffic-core, pure Rust)
  └─ GameDesign + ordered ServerEvent replay -> MatchSnapshot
```

Use authoritative server simulation. Clients may optimistically preview an event, but only server-issued `ServerEvent`s establish game state. Each submitted event must include the client's known network log sequence; reject stale submissions rather than applying them to a newer board. Sequence numbers belong to `traffic-network`, not `traffic-core` snapshots.

Use a room/game actor (one Tokio task owning one in-memory match) rather than sharing mutable `MatchSnapshot` behind locks. The actor serializes commands, validates the acting player and known sequence, applies `traffic-core`, persists one accepted `ServerEvent`, then broadcasts it. This prevents turn races and keeps reconnect behavior understandable.

Persist an append-only `Vec<ServerEvent>` per game. Its zero-based vector index is its log sequence. Broadcast accepted `Intend`, `Fulfil`, `EndTurn`, and open-turn `Undo` events to all participants. The shared rules engine derives planted/evicted status; clients never submit or receive evictions as events. A future server-only checkpoint is optional optimization, never the canonical record or a required client payload.

A `MatchSnapshot` contains only the current board, player, and positional active card. The room actor validates each prospective event against its authoritative current snapshot before appending it. Invalid events do not mutate state or enter the log. `Undo` is limited to the unended current turn; `EndTurn` commits that turn. A consensual cross-turn take-back is a future, distinct feature. The precise mechanics are specified in `docs/agent/rules.md` and `docs/agent/implementation.md`.

## Current local demo

The local demo is intentionally in-memory: it creates private two-seat rooms,
uses separate anonymous cookie sessions, and loses all rooms on server restart.
Run instructions are in `docs/agent/local-demo.md`. It serves a Trunk-built WASM
bundle from `crates/traffic-server/static/`, which is generated and ignored.
Use `cargo run -p traffic-server -- --trace-protocol` only for local protocol
diagnostics; it must never log cookies or session identifiers.

The browser interface is presently a functional transitional demo with
coordinate-labelled piles and arrow controls. The target graphical lattice,
rendered card stacks, automatic reconnect/catch-up, and client-local
step-back/step-forward review are specified but not yet implemented. The
browser currently requests a replay bootstrap on connection and retains the
received event log to reconstruct its snapshot.

## Suggested dependencies

- `traffic-core`: `serde`, `thiserror`. Do not add randomness unless a future game variant genuinely requires it; any such randomness must be explicitly seeded and preserved in game metadata.
- `traffic-network`: `serde`, `traffic-core`; keep it transport-neutral.
- `traffic-server`: `axum`, `tokio`, `tower-http`, `sqlx` (PostgreSQL), `argon2`, `uuid`, `tracing`, `tracing-subscriber`.
- `traffic-web`: `leptos` (CSR), `leptos_router`, `gloo-net` or a Rust-native browser WebSocket wrapper, `wasm-bindgen`, `web-sys` only at the browser boundary.

Pin compatible versions in the workspace `Cargo.toml`. Prefer server-rendered static shell only if SEO/public rules pages need it; the game itself should be a WASM client backed by WebSockets.

## Domain modeling rules

- Make illegal game states unrepresentable with enums and small newtypes (`PlayerId`, `Rank`, `PileHeight`, `RoomCode`, `LogSequence`).
- Model `ServerEvent` as a tagged enum and make replay deterministic and side-effect free.
- Keep `MatchSnapshot` opaque. Reconstruct it from `GameDesign` plus ordered `ServerEvent`s; do not transmit mutable state as a client-authoritative snapshot.
- Every rules addition requires table-driven unit tests in `traffic-core`; test invalid actions as carefully as valid ones.
- Never trust client-provided player identity, board state, turn status, intent legality, score, or game result.

## HTTP and WebSocket guidance

Keep HTTP for identity, room discovery/creation, static assets, health checks, and game-history pages. Use WebSockets for lobby changes and live play.

Use a typed protocol based on `traffic-network`:

```text
SubmitEvent { known_through, event }
CatchUpAfter { sequence }
RequestReplayBootstrap

SequencedEvent { sequence, event }
ReplayBootstrap { design, events }
```

Commands cover live turn actions (intend, fulfil, end turn, and undo). A stale duplicate is rejected because its `known_through` is no longer current. **Catch-up** means a memory-preserving reconnection that requests events after a retained sequence. **Replay bootstrap** means a memoryless authenticated re-entry that receives `GameDesign` and the complete log. Do not use client-facing game snapshots.

## Data and operations

- PostgreSQL is the source of truth for users, rooms, immutable game metadata, and ordered event logs.
- Treat in-memory room actors as a cache of active games. A server restart must recover games by replaying `GameDesign` and persisted events.
- Keep migrations in the repository and test them against an ephemeral PostgreSQL instance in CI.
- Provide structured logs with request/game/room/player correlation IDs. Never log authentication tokens or raw session cookies.
- Add rate limits for login, room creation, and WebSocket commands; impose message-size and connection limits.

## Quality gate

Before considering a change complete, run the narrowest relevant checks, and run the workspace checks for cross-cutting work:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For rules changes, include table-driven regression tests proving determinism, every action predicate, pile legality, derived eviction, settled-turn boundaries, and open-turn Undo. For protocol changes, test catch-up, replay bootstrap, stale duplicate commands, missing/out-of-order events, Undo, committed turns, and unauthorized commands.

## Working conventions

- Keep pull requests small and avoid mixing formatting, dependency upgrades, and behavior changes.
- Do not add rule logic to `traffic-server` or `traffic-web`.
- Preserve the declarative rules in the client. The UI may preview an attempted operation, but only the authoritative server accepts an event; rejected events must leave the local authoritative snapshot unchanged and visibly explain the rejection.
- Prefer explicit error types over stringly errors and avoid `unwrap`/`expect` outside tests or truly impossible invariant boundaries.
- Explain any database schema, protocol, or dependency decision in a short architecture record under `docs/adr/` once the project has those directories.
