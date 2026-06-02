# OurStory — MapleStory Tooling

A Rust workspace for MapleStory game tooling.

## Crates

- **`wz`** (`crates/wz/`) — Core WZ file parsing library. Wraps `wz_reader` with a typed `Node` API, canonical coordinate system (Y-up), and `TryFrom` impls for scalars, `DynamicImage`, and `Vector2D`. Bevy-independent; usable by client, server, and tooling.
- **`wz-cli`** (`crates/wz-cli/`) — CLI tool and library for probing MapleStory `.wz` asset files. Use this to explore the WZ tree structure (maps, sprites, items, sounds, strings, etc.), search nodes by name, dump subtrees as JSON, and understand the game data taxonomy.
- **`client`** (`crates/client/`) — Bevy-based game client that renders maps and sprites from WZ assets. Depends on `wz` for parsing; wraps types into Bevy via trivial `From` conversions.

## Server Crates

- **`protocol`** (`crates/protocol/`) — Shared packet definitions, opcodes, and game enums used by both client and server. No game logic.
- **`server-core`** (`crates/server-core/`) — Game logic library: map, channel, world, player state, routing. Pure logic, no I/O.
- **`server`** (`crates/server/`) — Binary that wires everything together. Accepts `--role login|world|channel|map` to run as the appropriate server type.

## Architecture

- **Channel owns TCP** — players connect to a Channel server. Map switches within a channel are in-memory (no reconnect).
- **Channel switch = TCP reconnect** — player disconnects from one channel port and reconnects to another. Buffs are preserved in-memory via World; everything else reloaded from DB.
- **MapHandle trait** — maps run in-process by default (`LocalMapHandle`). For high-population content (boss fights), a `RemoteMapHandle` impl forwards to a standalone map process.
- **Deployment** — one binary, four roles. Dev uses `--role channel` with all layers in-process. Prod can split Login, World, Channel(s), and standalone Map instances as needed.

## Coordinate System

WZ stores 2D pixel coordinates with Y increasing downward; Bevy uses Y-up.
All WZ→Bevy conversion happens at the `crates/wz/src/lib.rs` boundary:

- `TryFrom<Node> for Vector2D` reads WZ `Vector2D` values and negates Y.
- `Node::read_pos()` reads scalar `x`/`y` children and negates Y.
- `Node::read_pos_n(n)` reads `x{n}`/`y{n}` (footholds, areas) and negates Y.

Downstream consumers (`map`, `mob`, `character`, all `WzMapAsset` / `WzMobAsset`
fields, all `Transform`s, all events) treat coordinates as native Bevy-space.
The conversion formula `bevy_y = -wz_y` is applied exactly once per value,
inside the `wz` crate. There are no Y-negations or origin-flip sign games
in any runtime system. The client crate performs only trivial field copies
(`Vector2D(i32,i32)` → `Vec2(f32,f32)` with no sign changes).

**Origins** are loaded as Bevy-local pixel offsets (already Y-flipped). With
`Anchor::TOP_LEFT`, the formula `bevy_translation = pos - origin` places the
sprite's WZ pivot at the desired Bevy world position.

**Non-coordinate scalars** (`alpha`, `rx`, `ry`, `mag`, `delay`, `cy`,
`mobTime`, `force`, `piece`, `cx`, layer indices) are read as raw `i32`/`f32`
and untouched by the conversion.

**Footholds:** `Foothold.{x1,y1,x2,y2}` are Bevy-space. `layer_at()` uses
the inequality `f.y_at(x) >= y - 50.0` (foothold at or below entity, where
"below" means smaller Bevy Y).

**Network boundary** (future): `protocol::types::Position` uses WZ-Y
convention for wire compatibility with classic clients. Any code that
consumes inbound positions or emits outbound positions must negate Y at the
network handler. The server stores `Position` opaquely; it has no Y-direction
logic today.

## Client — Character Rendering (`crates/client/src/character/`)

The character module composites MapleStory character sprites (body, head, hair, face, equipment) with animation and correct z-ordering.

### Key components

- **Z-ordering** — `Base.wz/zmap.img` defines 151 z-layer names in file order. Lower zmap index = front (higher Bevy z). `ZMap::depth()` inverts the index: `bevy_z = (150 - index) + 50`, giving a z-range of 50–200 (above map tiles at z=0–2). `load_zmap()` uses `WzImage::resolve_children()` for deterministic file-order iteration (WZ reader stores children in a HashMap).
- **Hierarchical positioning** — `compute_frame_transforms()` positions parts via connection points from each part's `map` subnode (navel, neck, brow, hand, earOverHead, earBelowHead). Parts with `navel` attach to root center; others match by connection-point name to already-positioned parts.
- **Animation** — Body actions (stand1, walk1, jump, etc.) and face expressions (default, blink, etc.) animate independently via separate timers. Face expressions merge into the body frame's part list before transform computation.
- **Preloading** — All action frames for a body skin are preloaded at spawn. No lazy loading.
- **Keyboard controls** (dev testing) — `1` stand1, `2` walk1, `3` jump, `4` sit, `5` prone, `6` ladder, `7` rope, `8` fly, `9` alert, `0` dead, `Q` swingO1, `W` swingP1, `E` shoot1, `R` magic1.

### File structure

```
crates/client/src/character/
  mod.rs          — CharacterPlugin (registers observers + systems)
  components.rs   — CharacterRoot, CharacterConfig, CharacterAnimation, etc.
  events.rs       — SpawnCharacter, SetAction events
  types.rs        — ZMap, EquipSlot, SpriteLayer, FrameData, compute_frame_transforms
  loader.rs       — WzSpriteCache, preload_character_frames, load_part, load_frame
  systems.rs      — spawn_character, animate_characters, on_set_action, character_action_controls
```
