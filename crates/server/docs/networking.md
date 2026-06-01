# Networking

The networking layer is the boundary between TCP bytes and `GameEvent`s. It runs entirely on tokio. Three files: `codec.rs`, `session.rs`, `listener.rs`.

## Architecture

```
TcpListener::accept() → spawn session::run(socket, events_tx)
                              │
                              ├─ read half → Framed<ReadHalf, MapleCodec> → GameEvent::Packet
                              │
                              └─ write half ← mpsc<Vec<u8>> ← outbound queue
```

Each connected player is one tokio task. The task owns its socket split into read/write halves and runs until the client disconnects or the codec fails.

## Files

### `codec.rs` — `MapleCodec`

Length-prefixed framing. No encryption yet (TODO).

**Frame format:**

```
+----------------+----------------+---------------------+
| u16 LE length  | u16 LE opcode  | payload (length - 2)|
+----------------+----------------+---------------------+
```

`Decoder::decode` accumulates bytes in a `BytesMut`, returns a frame only when a complete packet is available. `Encoder::encode` prepends the length and opcode.

Implemented using `tokio_util::codec::{Decoder, Encoder}`. The `Framed` wrapper handles the buffer state machine.

### `session.rs` — per-connection task

`run(socket, events_tx)` is the per-connection task body. It:

1. Splits the `TcpStream` into `OwnedReadHalf` + `OwnedWriteHalf`
2. Wraps the read half in `Framed<ReadHalf, MapleCodec>`
3. Spawns a writer task that drains the outbound `mpsc::Receiver<Vec<u8>>`
4. Loops on `framed.next().await`:
   - On `(opcode, payload)`, sends `GameEvent::Packet` to the dispatcher
   - On `Err`, breaks the loop
5. On exit, sends `GameEvent::Disconnected` so the dispatcher can clean up

`Session` struct is currently a placeholder. The session is identified by `SessionId` (an atomic counter) for now; later it will hold the authenticated account, current character, etc.

### `listener.rs` — accept loop

`run(listener, events_tx)` accepts connections in a loop and spawns a session task per connection. Each channel server has one listener (one per `--role channel` port).

## Concurrency Model

- **N session tasks** running concurrently on the tokio runtime
- **1 dispatcher task** receiving all events through one `mpsc::Sender` (cloned per session)
- **N writer tasks** (one per session), each draining its own `mpsc::Receiver<Vec<u8>>`

Tasks communicate through channels, not shared state. The only shared state is `Arc<RwLock<World>>`, which only the dispatcher touches.

## Why a separate writer task?

The session task needs to:
- Read packets (concurrent with writing)
- Write packets (concurrent with reading)

`tokio::net::TcpStream::into_split` gives independent read/write halves. The session task owns the read half (driven by `Framed`); the writer task owns the write half (driven by the outbound `mpsc`). This is the standard "split ownership" pattern in tokio.

## What it does NOT do

- **No packet parsing.** The codec returns `(opcode, payload)` as raw bytes. Parsing is the handler's job.
- **No encryption.** `MapleCodec` is plaintext. MapleStory uses a custom XOR + AES shuffle that needs a reference client to match.
- **No keepalive.** Disconnects are detected by TCP close or codec error only.
