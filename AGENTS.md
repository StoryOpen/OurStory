# OurStory — MapleStory Tooling

A Rust workspace for MapleStory game tooling.

## Crates

- **`wz`** (`crates/wz/`) — CLI tool and library for probing MapleStory `.wz` asset files. Use this to explore the WZ tree structure (maps, sprites, items, sounds, strings, etc.), search nodes by name, dump subtrees as JSON, and understand the game data taxonomy. Forms the data layer for higher-level tooling.
- **`client`** (`crates/client/`) — Bevy-based game client that renders maps and sprites from WZ assets.
