# OurStory — MapleStory Tooling

A Rust workspace for MapleStory game tooling.

## Crates

- **`wz`** (`crates/wz/`) — CLI tool and library for probing MapleStory `.wz` asset files. Use this to explore the WZ tree structure (maps, sprites, items, sounds, strings, etc.), search nodes by name, dump subtrees as JSON, and understand the game data taxonomy. Forms the data layer for higher-level tooling.
- **`client`** (`crates/client/`) — Bevy-based game client that renders maps and sprites from WZ assets.

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

All entities in the WZ system (maps, mobs, NPCs, characters, etc.) share the same coordinate convention:

- **WZ X**: increases to the right (same as Bevy X)
- **WZ Y**: increases **downward** (opposite of Bevy Y, which increases upward)
- **origin**: a `Vector2D` pivot point within a sprite, relative to its top-left corner

When converting from WZ coordinates to Bevy world coordinates:

```
bevy_x = wz_x - origin.x
bevy_y = -wz_y + origin.y
```

This formula applies to map tiles, map objects (obj), mob sprites, NPC sprites, character sprites — every entity type. The origin is always subtracted from the WZ position, and WZ Y is negated for Bevy's Y-up convention.

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
