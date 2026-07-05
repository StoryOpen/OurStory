## Coordinate System

Use the following methods to read position, offsets, vectors, etc when applicable

- `TryFrom<Node> for Vector2D` reads WZ `Vector2D` values
- `Node::read_pos()` reads scalar `x`/`y` children
- `Node::read_origin()` reads Vector2D for the origin of the sprite
- `Node::read_pos_n(n)` reads `x{n}`/`y{n}` (footholds, areas)

**Origins** are local pixel offsets from the sprite's bottom-left corner. 

**Footholds:** `Foothold.{x1,y1,x2,y2}` are world-space. 


## Client — Character Rendering (`crates/client/src/character/`)

The character module composites MapleStory character sprites (body, head, hair, face, equipment) with animation and correct z-ordering.

### Key components

- **Z-ordering** — `Base.wz/zmap.img` defines 151 z-layer names in file order. Lower zmap index = front (higher Bevy z). `ZMap::depth()` inverts the index: `bevy_z = (150 - index) + 50`, giving a z-range of 50–200 (above map tiles at z=0–2). `load_zmap()` uses `WzImage::resolve_children()` for deterministic file-order iteration (WZ reader stores children in a HashMap).


## Remote Inspection (BRP)

The Bevy Remote Protocol (BRP) is enabled on the client binary. When running, it
listens at `http://127.0.0.1:15702` for JSON-RPC 2.0 requests. This allows
inspecting and modifying ECS state from external tools.

**Using from outside:**
```bash
# Must use --noproxy '*' to bypass the HTTP proxy at 127.0.0.1:8889
# List all entities with their Transform component
curl -s --noproxy '*' -X POST http://127.0.0.1:15702 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"world.query","id":1,"params":{"data":{"components":["bevy_transform::components::transform::Transform"]}}}'

# List all registered component types
curl -s --noproxy '*' -X POST http://127.0.0.1:15702 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"world.list_components","id":1}'
```

**Available methods:** `world.query`, `world.get_components`, `world.spawn_entity`,
`world.despawn_entity`, `world.insert_components`, `world.remove_components`,
`world.mutate_components`, `world.reparent_entities`, `world.list_components`,
`world.get_resources`, `world.insert_resources`, `world.remove_resources`,
`world.mutate_resources`, `world.list_resources`, `world.trigger_event`,
`world.write_message`, `registry.schema`, `schedule.list`, `schedule.graph`,
`rpc.discover`. Append `+watch` to streaming methods for SSE.

**Component types** use fully-qualified paths (e.g. `bevy_transform::components::transform::Transform`).
Custom types must derive `Reflect` and be registered with `app.register_type::<T>()`.

> **Startup latency:** The BRP HTTP server binds only after Bevy completes its
> initialization (renderer, window, asset pipeline, `Startup` systems) and the
> frame loop begins. This typically takes 1–5 seconds depending on hardware and
> WZ asset load. Attempting to `curl` the endpoint before then yields
> `Connection refused` — wait for the client's frame loop to start before
> querying BRP.

## Deployment — Dev & Prod Coexistence

Two environments run side-by-side on the same OCI VM:

| | Prod | Dev |
|---|---|---|
| **wz-server port** | `127.0.0.1:3000` | `127.0.0.1:3001` |
| **wz-server binary** | `/home/ubuntu/.cargo/bin/wz-server` | `/home/ubuntu/.cargo/bin/wz-server-dev` |
| **wz-server systemd** | `wz-server.service` | `wz-server-dev.service` |
| **wasm client dir** | `/home/ubuntu/www/` | `/home/ubuntu/www-dev/` |
| **nginx path** | `/` (root) | `/dev/` |
| **API path** | `/wz/...` | `/dev-wz/...` |

### Dev deployment script

`scripts/deploy-dev.sh` builds and deploys both artifacts to the dev slots on the VM.

Always use these exact values — no need to ask or confirm:
```bash
export OCI_VM_HOST="213.35.123.95"
export OCI_VM_SSH_KEY="$HOME/.ssh/oci_free_key"
./scripts/deploy-dev.sh
```

The script:
1. Cross-compiles `wz-server` for `aarch64-unknown-linux-musl` using `rust-lld` — **no external C cross-compiler needed**
2. Builds the wasm client for `wasm32-unknown-unknown` + runs `wasm-bindgen`
3. Injects `<base href="/dev/">` into the wasm client's `index.html`
4. SCPs the binary and wasm tarball to the VM
5. On the VM: installs the binary, restarts `wz-server-dev.service`, extracts wasm files to `/home/ubuntu/www-dev/`, and updates/reloads nginx

The one-time setup (systemd unit, nginx locations) is handled automatically and idempotently.

### Prerequisites

- `wasm-bindgen-cli` (`cargo install wasm-bindgen-cli`)
- SSH access to the OCI VM
- The `aarch64-unknown-linux-musl` target is auto-installed by the script
- No external C cross-compiler is required — `rust-lld` handles the link step

### Cross-compilation strategy

`wz-server` uses `aarch64-unknown-linux-musl` instead of `aarch64-unknown-linux-gnu` because:
- **No external linker needed** — `rust-lld` (ships with rustup) can link musl targets
- **Statically linked** — the binary bundles its own libc, zero runtime dependencies on the VM
- The binary is ~1.8 MB, same as the glibc version

### Release deployment

Push a `v*` tag to trigger the GitHub Actions workflows (`.github/workflows/deploy-server.yml` and `deploy-wasm-client.yml`), which deploy to the **prod** slots.

---

