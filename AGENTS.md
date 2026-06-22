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

## In-Game Inspector (`bevy-inspector-egui`)

In addition to BRP, the client includes an in-process egui-based entity
inspector via `bevy-inspector-egui` (using the
[`taboky-dev` fork](https://github.com/taboky-dev/bevy-inspector-egui) for
Bevy 0.19 compat).

Components must derive `Reflect` (same as BRP) to appear in the inspector.
No `register_type` call is needed for the inspector to discover them (Bevy's
`reflect_auto_register` handles that), but calling `register_type` is still
required for BRP visibility.

No connection is needed — the inspector renders inside the game window
itself, so there is no startup-latency issue as with BRP.
