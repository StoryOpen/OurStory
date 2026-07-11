//! Declarative WZ map loader.
//!
//! The data shape for `Map/Map/Map1/<id>.img` is declared here with
//! `#[derive(WzAsset)]` instead of being walked procedurally. Each field maps
//! to a child node via snake_case→camelCase (or `#[wz(rename = "…")]` /
//! `#[wz(child = "…")]`), and the generated `wz_build` recursively walks
//! the tree. The generic `WzAssetLoader<WzMapAsset>` turns a `wz://…/<id>.img.map`
//! handle into this struct.

use bevy::prelude::*;
use bevy_asset::RenderAssetUsages;
use wz::Vector2D;
use wz_derive::WzAsset;

#[derive(Asset, TypePath, Clone, Debug, WzAsset)]
#[wz(ext = "map", path = "Map/Map/Map1/{id}.img")]
pub struct WzMapAsset {
    pub info: MapInfo,
    // Direct numeric children 0..7 of the image node.
    #[wz(children(numeric_only))]
    pub layers: Vec<MapLayer>,
    #[wz(child = "back")]
    pub back: Vec<MapBack>,
    #[wz(child = "life")]
    pub life: Vec<MapLife>,
    #[wz(child = "portal")]
    pub portal: Vec<MapPortal>,
    #[wz(child = "ladderRope")]
    pub ladder_rope: Vec<MapLadderRope>,
    // seat/<int> leaves are raw vectors: now expressible via the TryFromNode fallback.
    #[wz(child = "seat")]
    pub seats: Vec<Vector2D>,
    // foothold/<layer>/<group>/<id> → 3-level numeric nesting.
    #[wz(child = "foothold")]
    pub foothold: Vec<MapFootholdLayer>,
    #[wz(child = "miniMap")]
    pub mini_map: MapMiniMap,
}

#[derive(Clone, Debug, WzAsset)]
pub struct MapInfo {
    pub bgm: String,
    pub cloud: i32,
    pub forced_return: i32,
    pub hide_minimap: i32,
    pub map_desc: String,
    pub map_mark: String,
    pub mob_rate: f32,
    pub move_limit: i32,
    pub on_user_enter: String,
    pub return_map: i32,
    pub town: i32,
    pub version: i32,
}

#[derive(Clone, Debug, WzAsset)]
pub struct MapLayer {
    pub info: MapLayerInfo,
    #[wz(child = "obj")]
    pub obj: Vec<MapObj>,
    #[wz(child = "tile")]
    pub tile: Vec<MapTile>,
}

#[derive(Clone, Debug, WzAsset)]
pub struct MapLayerInfo {
    #[wz(rename = "tS")]
    pub t_s: Option<String>,
}

#[derive(Clone, Debug, WzAsset)]
pub struct MapObj {
    pub f: i32,
    pub l0: String,
    pub l1: String,
    pub l2: String,
    #[wz(rename = "oS")]
    pub os: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub z_m: i32,
}

#[derive(Clone, Debug, WzAsset)]
pub struct MapTile {
    pub no: i32,
    pub u: String,
    pub x: i32,
    pub y: i32,
    pub z_m: i32,
}

#[derive(Clone, Debug, WzAsset)]
pub struct MapBack {
    pub a: i32,
    pub ani: i32,
    pub b_s: String,
    pub cx: i32,
    pub cy: i32,
    pub f: i32,
    pub front: i32,
    pub no: i32,
    pub rx: i32,
    pub ry: i32,
    #[wz(rename = "type")]
    pub kind: i32,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, WzAsset)]
pub struct MapLife {
    pub cy: i32,
    pub f: i32,
    pub fh: i32,
    pub hide: i32,
    pub id: String,
    pub mob_time: i32,
    pub rx0: i32,
    pub rx1: i32,
    #[wz(rename = "type")]
    pub kind: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, WzAsset)]
pub struct MapPortal {
    pub pn: String,
    pub pt: i32,
    pub tm: i32,
    pub tn: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, WzAsset)]
pub struct MapLadderRope {
    pub l: i32,
    pub page: i32,
    pub uf: i32,
    pub x: i32,
    pub y1: i32,
    pub y2: i32,
}

#[derive(Clone, Debug, WzAsset)]
pub struct MapFootholdLayer {
    #[wz(children(numeric_only))]
    pub groups: Vec<MapFootholdGroup>,
}

#[derive(Clone, Debug, WzAsset)]
pub struct MapFootholdGroup {
    #[wz(children(numeric_only))]
    pub segments: Vec<MapFootholdSegment>,
}

#[derive(Clone, Debug, WzAsset)]
pub struct MapFootholdSegment {
    pub next: i32,
    pub prev: i32,
    pub x1: i32,
    pub x2: i32,
    pub y1: i32,
    pub y2: i32,
}

#[derive(Clone, Debug, WzAsset)]
pub struct MapMiniMap {
    #[wz(skip)]
    pub canvas: Handle<Image>,
    pub center_x: i32,
    pub center_y: i32,
    pub height: i32,
    pub mag: i32,
    pub width: i32,
}
