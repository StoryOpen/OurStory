# Packet Flow

End-to-end trace of a single packet from TCP bytes to game state mutation, and the response path back.

## Inbound: TCP → Handler

```
                ┌─────────────┐
   TCP bytes →  │  Framed      │  (tokio-util codec, accumulates bytes)
                │  <ReadHalf,  │
                │  MapleCodec> │
                └──────┬───────┘
                       │ (opcode: u16, payload: Bytes)
                       ▼
                ┌─────────────┐
                │  Session     │  (per-connection task)
                │  task        │
                └──────┬───────┘
                       │ events_tx.send(GameEvent::Packet { session, opcode, payload })
                       ▼
                ┌─────────────┐
                │  mpsc        │  (channel of GameEvent, capacity 1024)
                └──────┬───────┘
                       │ events: mpsc::Receiver<GameEvent>
                       ▼
                ┌─────────────┐
                │  Dispatcher  │  (single task, owns Arc<RwLock<World>>)
                │  task        │
                └──────┬───────┘
                       │ world.write().await
                       ▼
                ┌─────────────┐
                │  handlers::  │  (match opcode, call handler fn)
                │  handle_packet│
                └──────┬───────┘
                       │ &mut World
                       ▼
                ┌─────────────┐
                │  World /     │  (sync game logic, no I/O, no .await)
                │  Channel /   │
                │  Map         │
                └─────────────┘
```

## Outbound: Handler → TCP

The outbound path is not yet wired. The pieces exist but the dispatcher doesn't yet enqueue responses. The intended path:

```
handlers::handle_packet(...) → builds response Vec<u8>
                              → SessionRegistry.send(session_id, packet)
                              → outbound_tx.try_send(packet)
                              → writer task in session::run
                              → write_half.write_all(&packet)
                              → TCP bytes
```

The `SessionRegistry` in `dispatcher.rs` tracks `SessionId → OutboundTx`. Handlers will look up the outbound channel for a given session and push to it.

## Why a single mpsc channel (not per-session)?

All session tasks share one `mpsc::Sender<GameEvent>` (cloned from the original). The dispatcher owns the single `mpsc::Receiver`. This means:

- The dispatcher processes events in arrival order (fair)
- A misbehaving session can flood the channel and starve others — this is a known limitation, mitigated by the 1024 capacity
- A simpler model: no routing logic, the dispatcher just pulls

Alternative would be per-session channels + a `select!` loop in the dispatcher. Tradeoff: explicit routing vs FIFO fairness. The current model favors simplicity.

## Event Types

```rust
pub enum GameEvent {
    Packet { session: SessionId, opcode: u16, payload: Vec<u8> },
    Disconnected { session: SessionId },
    PlayerLoaded { session: SessionId, char_id: i32 },
}
```

- `Packet` — every received packet becomes one
- `Disconnected` — session task sends on TCP close so the dispatcher can clean up
- `PlayerLoaded` — placeholder for async DB results; not yet produced by anything

## Opcode → Handler

`handlers::mod.rs::handle_packet` decodes the opcode via `protocol::opcodes::RecvOpcode::from_u16` and dispatches:

| Opcode | Handler |
|---|---|
| `LoginPassword` | `handlers::login::handle` (stub) |
| `CharacterSelect` | `handlers::character::handle_select` (stub) |
| `PlayerMove` | `handlers::movement::handle` (stub) |
| _other_ | no-op |

Handlers are async fns that take `&mut World` + `&Arc<dyn Database>` + `SessionId` + `&[u8]`. They can `.await` DB queries — the world write lock is held across the await, but DB calls are fast and the lock is released as soon as the handler returns.

## Lock Duration

The world write lock is held only for the duration of `handle_packet`. For a typical packet (e.g., a movement update with no DB call), this is microseconds. Even with a DB call, it's a single round-trip (~1-5ms). All 800 player sessions on a channel share this one write lock — sequential packet processing.

The alternative — finer-grained locks per channel or per map — was rejected for the initial implementation. The workload is event-driven with low packet rates (1-5/sec per player), so contention is negligible.
