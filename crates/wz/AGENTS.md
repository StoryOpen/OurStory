# WZ Crate — MapleStory WZ File Probe

## Overview

This crate reads MapleStory `.wz` files using `wz_reader` (v0.0.20, by spd789562). WZ files contain all game assets: maps, sprites, sounds, strings, skills, items, NPCs, mobs, quests, UI, etc. The WZ format encodes a tree structure where each node has a name, object type, optional children, and optional value data.

## WZ Files

All 17 `.wz` files live in `wz/` at workspace root. `Base.wz` is loaded first; it references the rest:

```
Base.wz  →  Character.wz, Map.wz, Item.wz, Mob.wz, Npc.wz, Skill.wz,
             String.wz, Sound.wz, UI.wz, Etc.wz, Effect.wz, Quest.wz,
             Reactor.wz, Morph.wz, TamingMob.wz
```

## Tree Structure

### Node types (`WzObjectType`)

- **File** — top-level WZ file reference (e.g. `Character`, `Map`)
- **Directory** — intermediate directory within a WZ archive
- **Image** (`.img`) — a parsed image/property container; **must be parsed** before children are accessible
- **Property** — intermediate property node with named children
- **Value** — leaf node with a primitive value
- **MsFile / MsImage** — MapleStory-specific variants (rare)

### Value types a leaf node can hold

- `i32` (int), `i16` (short), `i64` (long)
- `f32` (float), `f64` (double)
- `String` (via `get_string()`)
- `Vector2D { x, y }` (via `try_as_vector2d`)
- `WzPng` — embedded PNG sprite (via `try_as_png`)
- `WzSound` — audio data
- `WzLua` — Lua script
- `WzVideo` — video data
- `WzRawData` — raw binary blob

## Known Taxonomy (discovered by probing)

### Top level (`wz list`)

```
Character  (35 children)  — character sprites by job ID
UI         (19)           — UI windows, buttons, components
Item       (6)            — Cash, Consume, Etc, Install, Pet, Special
Etc        (22)           — scripts, chat, map helper, etc.
Effect     (17)           — visual effects
Map        (8)            — maps, tiles, objects, backgrounds
Morph      (42)           — transformation/mount sprites
TamingMob  (7)            — mount/pet data
Mob        (1565)         — monster data
Quest      (6)            — quest dialogs, checks
Npc        (1620)         — NPC sprites
String     (20)           — names/descs for items, maps, mobs, skills
Sound      (44)           — BGM, SFX
Reactor    (419)          — map reactor objects
Skill      (76)           — skill data by class ID
```

### Map.wz structure

```
Map/
  Map/
    Map0..Map9/          — world maps by ID
      100000000.img/     — Henesys town
        info/            — metadata: bgm, mobRate, town, returnMap, fieldLimit, etc.
        back/            — background layers (0..N with front/parallax/type/etc.)
        life/            — NPCs/mobs placed on map
          0/             — { id, type, x, y, cy, fh, rx0, rx1, mobTime }
        portal/          — portals
          0/             — { pt, pn, x, y, tm, tn, script }
        foothold/        — physics walkable surfaces
          0/             — { x1, y1, x2, y2, force, forbidFall, piece }
        ladderRope/      — ladders/ropes { x, y1, y2, l, page }
        seat/            — chair positions { x, y }
        area/            — rectangular zones
        miniMap/         — { canvas (PNG), width, height, centerX, centerY, mag }
        ToolTip/         — tooltip data
        0..7/            — map layers (tile + obj)
          info/          — layer metadata
          tile/          — map tiles { u, no, x, y, zM, mag }
          obj/           — placed objects { oS, l0, l1, l2, x, y, z, zM, f, r, type }
Map/
  Tile/                  — tile sprite sheets by tileset
  Obj/                   — object sprites by object set
  MapHelper.img/         — minimap marks, etc.
```

### Character.wz structure

```
Character/
  $jobID.img/            — e.g. 00002000.img (Dual Blade)
    info/                — { cash, islot, vslot }
    walk1/               — animation action
      0/                 — frame 0
        body             — PNG sprite subnode (with origin/head/臂等 children)
        arm              — PNG sprite subnode
        head             — PNG sprite
        face             — int animation property
        delay            — frame duration in ms
    stand1/              — standing frames
    jump/                — jumping frames
    ...                  — many actions (skills, movement, emotes, etc.)
```

### Mob.wz structure

```
Mob/
  $mobID.img/            — e.g. 0130100.img
    info/                — { level, maxHP, maxMP, exp, PADamage, PDDamage, speed,
                            elemAttr, acc, eva, pushed, bodyAttack, undead, etc. }
    stand/               — standing sprites
    move/                — movement sprites
    hit1/                — hit reaction
    die1/                — death animation
```

### Item.wz structure

```
Item/
  Cash/                  — cash shop items by category ID (0501.img, etc.)
  Consume/               — consumables (0200.img, etc.)
  Etc/                   — etc items (0400.img, etc.)
  Install/               — installation items (0301.img)
  Pet/                   — pet items (5000000.img, etc.)
  Special/               — special items (0900.img, 0910.img, MaplePoint.img)
```

### String.wz structure (name/desc lookups)

```
String/
  Eqp.img/
    Eqp/
      Cap/               — item ID → { name: "str", desc: "str" }
      Weapon/            — same pattern
      Coat/              — ...
      ...                — all equipment categories
  Map.img/               — map ID → { mapName, streetName }
  Mob.img/               — mob ID → { name }
  Skill.img/             — skill ID → { name, desc }
  Npc.img/               — NPC ID → { name }
  Consume.img/           — consumable names
  Cash.img/              — cash item names
  Ins.img/               — install item names
  Pet.img/               — pet names
  Etc.img/               — etc item names
```

### Skill.wz structure

```
Skill/
  $classID.img/          — e.g. 100.img (beginner)
    skill/
      $skillID/          — e.g. 1000000
        icon/            — PNG
        iconDisabled/    — PNG
        iconMouseOver/   — PNG
        level/           — per-level data
          1/             — { mpCon, damage, x, y, etc. }
          2/
          ...
```

### Npc.wz structure

```
Npc/
  $npcID.img/
    info/                — { link, hideName, ... }
    stand/               — standing frames
    move/                — movement
    say/                 — chat frames
```

## Important: .img Parsing

`.img` nodes are "lazy" — their children are not loaded until `parse_node()` is called. The library's `resolve_path()`, `get_children()`, `get_node_value_detail()`, `schema_tree()`, and `resolve_link_target()` all handle this automatically, but any direct access to `node.read().unwrap().children` on an unparsed `.img` node will return empty.

Use:
```rust
wz::parse_node(&node)?;  // explicit parse
// or just use get_children() / resolve_path() which handle it
```

## WZ Path Format

Paths use `/` as separator, relative to `Base.wz` root:
- `Map/Map/Map1/100000000.img/info/bgm` → value: `"Bgm00/FloralLife"`
- `Character/00002000.img/walk1/0/body` → PNG sprite
- `String/Eqp.img/Eqp/Cap/1003043/name` → value: `"순록의 뿔"`

## Library API

```rust
use std::path::Path;
use wz::*;

let base = load_base(Path::new("./wz"))?;

// Navigate
let node = resolve_path(&base, "Map/Map/Map1/100000000.img")?;

// Children
let children = get_children(&node); // Vec<(String, WzNodeArc)>

// Info
let info = get_node_info(&node); // NodeInfo { name, object_type, value, children_count, full_path }

// Tree (depth-limited)
let tree = collect_tree(&node, 3, 0); // TreeNode { info, children }

// Walk all
walk_nodes(&node, true, |n| { /* called for every node */ });

// --- New in v0.2 ---

// Get value type name for a node
let typ = value_type_name(&node); // "int", "string", "vector", "png", etc.

// Get detailed value (scalar, PNG with sub-properties, sound metadata, etc.)
if let Some(val) = get_node_value_detail(&node) {
    // val is a serde_json::Value — raw number/string for scalars,
    // or for PNG: {"type":"png","width":27,"height":32,"properties":{"origin":{"x":19,"y":32},...}}
}

// Resolve UOL / _inlink / _outlink references
if let Some(target) = resolve_link_target(&node) {
    let info = get_node_info(&target);
}

// Export PNG or sound to disk
export_node(&node, Path::new("./output"))?;

// Build a schema tree showing field names, types, and examples
let schema = schema_tree(&node, 2); // recursive, depth-limited
```

## CLI Usage

```sh
# Run from workspace root (wz/ dir must exist relative to CWD)
cargo run -p wz -- <command>

# List root
cargo run -p wz -- list

# List a path
cargo run -p wz -- list "Map/Map/Map1"

# Tree view
cargo run -p wz -- tree "Character/00002000.img" -d 2

# Detailed info (shows PNG dimensions & sub-properties for sprites)
cargo run -p wz -- info "String/Eqp.img/Eqp/Cap/1003043"
cargo run -p wz -- info "Character/00002000.img/walk1/0/body"

# JSON output (for pipelines / MCP)
cargo run -p wz -- info --json "Mob/0130100.img/info"

# Full JSON dump
cargo run -p wz -- dump "Map/Map/Map1/100000000.img/info"

# Search by name
cargo run -p wz -- search "Henesys"
cargo run -p wz -- search --json "100000000"

# --- New commands ---

# Get raw value (scalar, string, vector, or PNG metadata)
cargo run -p wz -- get "Mob/0130100.img/info/level"        # → 4
cargo run -p wz -- get "String/Eqp.img/Eqp/Cap/1003043/name"  # → "순록의 뿔"
cargo run -p wz -- get --json "Character/00002000.img/walk1/0/body/origin"  # → {"x":19,"y":32}

# Resolve UOL / _inlink / _outlink to target node
cargo run -p wz -- resolve "some/link/node"
cargo run -p wz -- resolve --json "some/link/node"

# Export PNG or sound to a directory
cargo run -p wz -- export "Character/00002000.img/walk1/0/body" -o ./sprites
cargo run -p wz -- export "Sound/Bgm00/FloralLife" -o ./sounds

# Show schema (field names, types, example values) at a path
cargo run -p wz -- schema "Mob/0130100.img/info" -d 2
cargo run -p wz -- schema --json "Character/00002000.img/walk1/0"
```

## wz_reader Version

Using `wz_reader = "0.0.20"` with `json` feature enabled (for serde serialization). Also depends on `image = "0.25"` with `png` feature for PNG export. The crate uses `Arc<RwLock<WzNode>>` internally — thread-safe, shared tree navigation. Key types re-exported by `wz_reader`:

| Type | Description |
|------|-------------|
| `WzNodeArc` | `Arc<RwLock<WzNode>>` — shared node handle |
| `WzNode` | Fields: `name`, `object_type`, `parent` (Weak), `children` (HashMap) |
| `WzNodeCast` | Trait with `try_as_*` methods for value extraction |
| `WzObjectType` | Enum: `File`, `Directory`, `Image`, `Property`, `Value`, etc. |
