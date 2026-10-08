# Traffic in Hanoi — Agent Guide

## Product intent

Build a browser-playable, real-time implementation of **Traffic in Hanoi**. Players create or join private rooms, play a complete game together, reconnect safely after a temporary disconnect, and can review the completed game.

The application is a **100% Rust stack**: Rust on the server and Rust compiled to WebAssembly in the browser. Do not introduce JavaScript or TypeScript application code. Small declarative JavaScript-free HTML and CSS assets are fine.

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
  └─ GameCreated + ordered GameEvent replay -> GameState
```

Use authoritative server simulation. Clients may optimistically preview a delta, but only server-issued `GameEvent`s establish game state. Each submitted delta must include the client's known `GameSequence`; reject stale submissions rather than applying them to a newer board.

Use a room/game actor (one Tokio task owning one in-memory game) rather than sharing mutable `GameState` behind locks. The actor serializes commands, validates the acting player and known sequence, applies `traffic-core`, persists one accepted `GameEvent`, then broadcasts it. This prevents turn races and keeps reconnect behavior understandable.

Persist an append-only game event log. Traffic in Hanoi has public board information: broadcast root selections, intentions, fulfilments, end attempts, and Undo live to all participants. The shared rules engine records per-turn history entries (`Root`, `Intention`, `Fulfilment`, `End`, and derived `Eviction`) in batches. The network carries player deltas and ordered events; clients never submit evictions directly. Clients retain and replay the complete log. A future server-only checkpoint is optional optimization, never the canonical record or a required client payload.

Treat an in-progress turn as a server-authoritative, reversible transaction. The server validates each appended history entry incrementally and derives eviction entries one at a time. A batch stops at its first invalid history position and is then blocked; no later operation is accepted until Undo removes that final batch. Do not treat the final turn submission as the sole rules-validation point. The precise mechanics are specified in `docs/rules.md` and `docs/implementation.md`.

## Current local demo

The local demo is intentionally in-memory: it creates private two-seat rooms,
uses separate anonymous cookie sessions, and loses all rooms on server restart.
Run instructions are in `docs/local-demo.md`. It serves a Trunk-built WASM
bundle from `crates/traffic-server/static/`, which is generated and ignored.
Use `cargo run -p traffic-server -- --trace-protocol` only for local protocol
diagnostics; it must never log cookies or session identifiers.

## Suggested dependencies

- `traffic-core`: `serde`, `thiserror`. Do not add randomness unless a future game variant genuinely requires it; any such randomness must be explicitly seeded and preserved in game metadata.
- `traffic-network`: `serde`, `traffic-core`; keep it transport-neutral.
- `traffic-server`: `axum`, `tokio`, `tower-http`, `sqlx` (PostgreSQL), `argon2`, `uuid`, `tracing`, `tracing-subscriber`.
- `traffic-web`: `leptos` (CSR), `leptos_router`, `gloo-net` or a Rust-native browser WebSocket wrapper, `wasm-bindgen`, `web-sys` only at the browser boundary.

Pin compatible versions in the workspace `Cargo.toml`. Prefer server-rendered static shell only if SEO/public rules pages need it; the game itself should be a WASM client backed by WebSockets.

## Domain modeling rules

- Make illegal game states unrepresentable with enums and small newtypes (`PlayerId`, `CardId`, `RoomCode`, `GameSequence`).
- Model `GameDelta` as a tagged enum and make replay deterministic and side-effect free.
- Keep `GameState` opaque. Reconstruct it from `GameCreated` plus ordered `GameEvent`s; do not transmit mutable state as a client-authoritative snapshot.
- Every rules addition requires table-driven unit tests in `traffic-core`; test invalid actions as carefully as valid ones.
- Never trust client-provided player identity, board state, turn status, intent legality, score, or game result.

## HTTP and WebSocket guidance

Keep HTTP for identity, room discovery/creation, static assets, health checks, and game-history pages. Use WebSockets for lobby changes and live play.

Use a typed protocol based on `traffic-network`:

```text
SubmitDelta { known_sequence, delta }
CatchUpAfter { sequence }
RequestReplayBootstrap

GameEvent { sequence, delta }
ReplayBootstrap { created, events }
```

Commands cover live turn actions (root, intention, fulfilment, undo, and end). A stale duplicate is rejected because its `known_sequence` is no longer current. **Catch-up** means a memory-preserving reconnection that requests events after a retained sequence. **Replay bootstrap** means a memoryless authenticated re-entry that receives `GameCreated` and the complete log. Do not use client-facing game snapshots.

## Data and operations

- PostgreSQL is the source of truth for users, rooms, immutable game metadata, and ordered event logs.
- Treat in-memory room actors as a cache of active games. A server restart must recover games by replaying `GameCreated` and persisted events.
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

For rules changes, include regression tests proving determinism and validating every intermediate history position. For protocol changes, test catch-up, replay bootstrap, stale duplicate commands, missing/out-of-order events, blocked batches, Undo, automatic effects, and unauthorized commands.

## Working conventions

- Keep pull requests small and avoid mixing formatting, dependency upgrades, and behavior changes.
- Do not add rule logic to `traffic-server` or `traffic-web`.
- Preserve the declarative rules in the client. The UI may accept an attempted operation that violates them, but must immediately create a visible blocked batch rather than silently adding gameplay guardrails or allowing later operations.
- Prefer explicit error types over stringly errors and avoid `unwrap`/`expect` outside tests or truly impossible invariant boundaries.
- Explain any database schema, protocol, or dependency decision in a short architecture record under `docs/adr/` once the project has those directories.
