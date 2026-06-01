# Dispatcher

The dispatcher is a single tokio task that owns `Arc<RwLock<World>>` and is the *only* code path that mutates game state. All network events flow through it.

## Why a Single Mutator?

Three options for sharing game state across async tasks:

1. **`Arc<Mutex<World>>`** — every task locks for every operation. Simple but serializes everything.
2. **Fine-grained locks** — `Arc<RwLock<Map>>` per map, `Arc<RwLock<Player>>` per player, etc. Less contention but more complexity (deadlock risk, partial updates).
3. **One mutator task** — `Arc<RwLock<World>>` is held by exactly one task. Other tasks send messages. No locks across tasks. All serialization is implicit (FIFO on the event channel).

This project uses option 3. It works because:

- MapleStory's packet rate is low (1-5/sec per player)
- Game logic is fast (microseconds per packet)
- One task with sequential processing is plenty fast for the workload

The cost: no parallelism in game logic. The benefit: no lock contention, no deadlock, no partial-update bugs.

## What the Dispatcher Owns

```rust
pub async fn run(
    world: Arc<RwLock<World>>,
    db: Arc<dyn Database>,
    mut events: mpsc::Receiver<GameEvent>,
) {
    let mut sessions = SessionRegistry::new();
    while let Some(event) = events.recv().await {
        match event {
            GameEvent::Packet { session, opcode, payload } => {
                let mut w = world.write().await;
                handlers::handle_packet(&mut w, &db, session, opcode, &payload).await;
            }
            GameEvent::Disconnected { session } => {
                sessions.remove(session);
                let mut w = world.write().await;
                handlers::handle_disconnect(&mut w, session);
            }
            GameEvent::PlayerLoaded { session, char_id } => {
                let mut w = world.write().await;
                if let Some(player) = db.load_player(char_id).await {
                    w.register_player(player);
                }
                let _ = session;
            }
        }
    }
}
```

- `world: Arc<RwLock<World>>` — passed by clone to wherever it's needed (just the dispatcher, currently)
- `db: Arc<dyn Database>` — passed by clone to handlers
- `events: mpsc::Receiver<GameEvent>` — the single inbound channel
- `sessions: SessionRegistry` — local state, not shared

## Session Registry

The dispatcher keeps a `HashMap<SessionId, OutboundTx>` so handlers can send responses back to specific sessions. This is the missing piece of the outbound path (see [Packet Flow](packet-flow.md)).

When a session task starts, it creates its own `mpsc::channel(256)` and gives the sender to the dispatcher via... currently nothing. This is the next thing to wire. The intended flow:

1. `session::run` creates the outbound channel and sends the `OutboundTx` to the dispatcher via a side channel (e.g., a `mpsc::Sender<SessionInit>`)
2. Dispatcher adds it to `SessionRegistry`
3. Handlers look up the `OutboundTx` by `SessionId` and call `try_send`

## Lock Held Across `.await` — Is That OK?

In `handle_packet` we have:

```rust
let mut w = world.write().await;     // acquire lock
handlers::handle_packet(&mut w, ...).await;  // hold lock across .await
                                            // lock released when handler returns
```

The world write guard `w` is held while the handler awaits DB calls. This is **deliberate**, not accidental:

- `tokio::sync::RwLock` write guards are not `Send` across `.await` by default — but they are, since `tokio::sync::RwLockWriteGuard` is `Send`
- The lock is held for the entire handler call, including any DB awaits
- The handler is expected to be fast — no long-running work

Why hold the lock across DB calls? Because the handler reads/writes world state, then queries the DB, then writes more world state based on the result. Splitting this would require either:
- Releasing the lock between DB calls (allow other handlers to interleave → inconsistent state)
- Loading all data before the lock (impossible if the data needed depends on world state)

The lock-held-across-await pattern is fine because:
- Handlers complete in milliseconds
- No other task is waiting on the read lock (the dispatcher is the only writer, and Tokio's RwLock is fair-ish)
- The "lock held during await" warning doesn't apply to `tokio::sync::RwLock` — its guard is `Send`

## Concurrency Summary

| Task | Reads | Writes |
|---|---|---|
| Session task (per connection) | — | outbound mpsc |
| Dispatcher | `world.read()` (not used yet) | `world.write().await` |
| Handler (called from dispatcher) | `&mut World` (passed in) | via `&mut` |

Only the dispatcher acquires the write lock. Handlers operate on `&mut World` obtained from the dispatcher. There is no path to `World` from outside the dispatcher task.

## When This Pattern Breaks

If a single channel ever needs to handle >5000 concurrent players, the single dispatcher becomes the bottleneck (every packet goes through one FIFO). Mitigations:

- **Per-channel dispatchers** — split `world` into `Arc<RwLock<Channel>>` per channel, one dispatcher per channel
- **Per-map dispatchers** — even finer; each map gets its own task
- **Sharded worlds** — multiple dispatchers, each owning a subset of channels

For the initial implementation (800 players per channel target), this is not needed. The architecture is designed to make those splits easy when they become necessary.
