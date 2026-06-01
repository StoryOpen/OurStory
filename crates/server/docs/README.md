# Server Architecture

The server is a single Rust binary that runs as one of four roles, selected by `--role`:

- `login` — login/auth server (port 8484, stub)
- `world` — world server coordinating channels (stub)
- `channel` — channel server, the hot path. Players connect here
- `map` — standalone map instance for high-population content (stub)

## Crate Layout

```
crates/
├── protocol/         # Packet opcodes, request/response types, shared enums
├── server-core/      # Game logic library: World, Channel, Map, Player, DB trait
└── server/           # Binary: tokio networking, sqlx DB, role dispatch
```

`server-core` exposes sync APIs (`&mut self` on World/Channel/Map). The `server` binary wraps it in `Arc<RwLock<...>>` and drives everything through a single async dispatcher task.

## Core Idea: One Mutator, Many Readers

Every byte that arrives from a player TCP socket becomes a `GameEvent` and is funneled into one task — the **dispatcher**. The dispatcher is the *only* code that mutates game state. Session tasks read packets, send events, and write outbound bytes — they never touch the world.

```
TCP socket → Session task (read) → mpsc<GameEvent> → Dispatcher task → Arc<RwLock<World>> → handlers
                                          ↓
                            Session task (write) ← mpsc<Vec<u8>>
```

This is enforced by:
1. `Arc<RwLock<World>>` lives only inside the dispatcher
2. `GameEvent` is the only way to influence game state from outside the dispatcher
3. Handlers take `&mut World` — they cannot be called without the dispatcher holding the write lock

## The Three Layers

| Layer | What runs | Async? | Location |
|---|---|---|---|
| **Network** | TCP accept, frame parsing, byte I/O | yes | `src/net/{listener,session,codec}.rs` |
| **Dispatcher** | All game state mutation, packet handlers | yes | `src/net/dispatcher.rs` |
| **Game logic** | World, Channel, Map, Player methods | sync (`&mut self`) | `server-core/src/` |

The network layer feeds events. The dispatcher applies them. Game logic does CPU work under a brief RwLock write — no I/O, no `.await` inside `&mut World` methods.

## Where to Read Next

- [Networking](networking.md) — session tasks, codec, listener
- [Packet Flow](packet-flow.md) — from TCP bytes to handler call
- [Dispatcher](dispatcher.md) — the single mutator pattern
- [Database](database.md) — sqlx/Postgres integration
