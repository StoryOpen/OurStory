use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
    reflect::TypePath,
};
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use image::DynamicImage;

use thiserror::Error;

use crate::wz::WzNodeExt;

#[derive(Asset, TypePath, Debug)]
pub struct WzMapAsset {
    pub info: MapInfo,
    pub objs: Vec<WzSpriteData>,
    pub tiles: Vec<WzSpriteData>,
    pub footholds: Vec<Foothold>,
    pub backgrounds: Vec<BackgroundData>,
    pub life: Vec<LifeSpawn>,
    pub portals: Vec<PortalData>,
    pub ladder_ropes: Vec<LadderRopeData>,
    pub seats: Vec<SeatData>,
    pub areas: Vec<AreaData>,
    pub minimap: Option<MiniMapData>,
}

#[derive(Debug, Clone, Default)]
pub struct MapInfo {
    pub bgm: Option<String>,
    pub cloud: Option<i32>,
    pub town: Option<i32>,
    pub return_map: Option<i32>,
    pub forced_return: Option<i32>,
    pub field_limit: Option<i32>,
    pub mob_rate: Option<f32>,
    pub fly: Option<i32>,
    pub move_limit: Option<i32>,
    pub lv_limit: Option<i32>,
    pub hide_minimap: Option<i32>,
    pub map_name: Option<String>,
    pub street_name: Option<String>,
    pub map_desc: Option<String>,
    pub on_first_user_enter: Option<String>,
    pub on_user_enter: Option<String>,
    pub time_limit: Option<i32>,
    pub field_type: Option<i32>,
    pub expedition_only: Option<i32>,
    pub party_only: Option<i32>,
}

#[derive(Debug)]
pub struct WzSpriteData {
    pub image: Handle<Image>,
    pub x: f32,
    pub y: f32,
    pub z: i32,
    pub z_m: i32,
    pub layer: u8,
    pub zid: i32,
    pub origin: Vec2,
}

#[derive(Debug, Clone, Copy)]
pub struct Foothold {
    pub id: i32,
    pub group: i32,
    pub layer: u8,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub force: Option<i32>,
    pub forbid_fall: Option<i32>,
    pub piece: Option<i32>,
}

impl Foothold {
    pub fn y_at(&self, x: f32) -> f32 {
        if (self.x2 - self.x1).abs() < f32::EPSILON {
            self.y1
        } else {
            let t = ((x - self.x1) / (self.x2 - self.x1)).clamp(0.0, 1.0);
            self.y1 + t * (self.y2 - self.y1)
        }
    }

    pub fn contains_x(&self, x: f32) -> bool {
        let lo = self.x1.min(self.x2);
        let hi = self.x1.max(self.x2);
        x >= lo && x <= hi
    }
}

pub fn layer_at(footholds: &[Foothold], x: f32, y: f32) -> Option<u8> {
    footholds
        .iter()
        .filter(|f| f.contains_x(x))
        .filter(|f| (f.y_at(x) - y).abs() < 300.0 && f.y_at(x) >= y - 50.0)
        .min_by(|a, b| {
            let da = (a.y_at(x) - y).abs();
            let db = (b.y_at(x) - y).abs();
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|f| f.layer)
}

#[derive(Debug)]
pub struct BackgroundData {
    pub image: Handle<Image>,
    pub front: bool,
    pub rx: i32,
    pub ry: i32,
    pub btype: i32,
    pub cx: i32,
    pub cy: i32,
    pub alpha: u8,
    pub x: f32,
    pub y: f32,
    pub origin: Vec2,
    pub index: i32,
}

#[derive(Debug)]
pub struct LifeSpawn {
    pub spawn_type: String,
    pub id: i32,
    pub x: f32,
    pub y: f32,
    pub cy: i32,
    pub fh: i32,
    pub rx0: i32,
    pub rx1: i32,
    pub mob_time: i32,
    pub hide: bool,
    pub flip: bool,
}

#[derive(Debug)]
pub struct PortalData {
    pub pt: i32,
    pub pn: String,
    pub x: f32,
    pub y: f32,
    pub tm: i32,
    pub tn: String,
    pub script: Option<String>,
    pub delay: Option<i32>,
    pub horizontal_impact: Option<i32>,
    pub vertical_impact: Option<i32>,
    pub only_once: Option<i32>,
}

#[derive(Debug)]
pub struct LadderRopeData {
    pub x: f32,
    pub y1: f32,
    pub y2: f32,
    pub is_ladder: bool,
    pub uf: i32,
    pub page: i32,
}

#[derive(Debug)]
pub struct SeatData {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug)]
pub struct AreaData {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

#[derive(Debug)]
pub struct MiniMapData {
    pub image: Handle<Image>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub mag: Option<i32>,
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum MapLoaderError {}

#[derive(Default, TypePath)]
pub struct WzMapLoader;

impl AssetLoader for WzMapLoader {
    type Asset = WzMapAsset;
    type Settings = ();
    type Error = MapLoaderError;

    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let asset_path = load_context.path().path().to_string_lossy();
        let wz_path = asset_path.strip_suffix(".map").unwrap_or(&asset_path);

        let base = crate::wz::get_cached_base();
        let map = base.at_path(wz_path).expect("map node not found");

        let info = load_info(&map);
        let footholds = load_footholds(&map);
        let mut tiles = load_tiles(&map, &base, load_context);
        let mut objs = load_objs(&map, &base, load_context);
        sort_sprites(&mut tiles);
        sort_sprites(&mut objs);
        let backgrounds = load_backgrounds(&map, &base, load_context);
        let life = load_life(&map);
        let portals = load_portals(&map);
        let ladder_ropes = load_ladder_ropes(&map);
        let seats = load_seats(&map);
        let areas = load_areas(&map);
        let minimap = load_minimap(&map, &base, load_context);

        Ok(WzMapAsset {
            info, objs, tiles, footholds, backgrounds,
            life, portals, ladder_ropes, seats, areas, minimap,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["map"]
    }
}

fn sort_sprites(sprites: &mut [WzSpriteData]) {
    sprites.sort_by(|a, b| {
        a.layer.cmp(&b.layer)
            .then_with(|| a.z.cmp(&b.z))
            .then_with(|| a.zid.cmp(&b.zid))
    });
}

fn load_info(map: &crate::wz::Node) -> MapInfo {
    let info_node = match map.at_path("info") {
        Ok(n) => n,
        Err(_) => return MapInfo::default(),
    };

    let s = |path: &str| info_node.get_opt::<String>(path);
    let i = |path: &str| info_node.get_opt::<i32>(path);
    let f = |path: &str| info_node.get_opt::<f32>(path);

    MapInfo {
        bgm: s("bgm"),
        cloud: i("cloud"),
        town: i("town"),
        return_map: i("returnMap"),
        forced_return: i("forcedReturn"),
        field_limit: i("fieldLimit"),
        mob_rate: f("mobRate"),
        fly: i("fly"),
        move_limit: i("moveLimit"),
        lv_limit: i("lvLimit"),
        hide_minimap: i("hideMinimap"),
        map_name: s("mapName"),
        street_name: s("streetName"),
        map_desc: s("mapDesc"),
        on_first_user_enter: s("onFirstUserEnter"),
        on_user_enter: s("onUserEnter"),
        time_limit: i("timeLimit"),
        field_type: i("fieldType"),
        expedition_only: i("expeditionOnly"),
        party_only: i("partyOnly"),
    }
}

fn load_footholds(map: &crate::wz::Node) -> Vec<Foothold> {
    let mut footholds = Vec::new();
    if let Ok(fh_root) = map.at_path("foothold") {
        for (layer_name, group_node) in fh_root.children() {
            let layer_num: u8 = layer_name.as_str().parse().unwrap_or(0);
            for (_group_name, id_node) in group_node.children() {
                for (id_name, fh) in id_node.children() {
                    let (x1, y1) = fh.read_pos_n(1).unwrap_or((0.0, 0.0));
                    let (x2, y2) = fh.read_pos_n(2).unwrap_or((0.0, 0.0));
                    let id: i32 = id_name.as_str().parse().unwrap_or(0);
                    let force: Option<i32> = fh.get_opt("force");
                    let forbid_fall: Option<i32> = fh.get_opt("forbidFall");
                    let piece: Option<i32> = fh.get_opt("piece");
                    footholds.push(Foothold { id, group: 0, layer: layer_num, x1, y1, x2, y2, force, forbid_fall, piece });
                }
            }
        }
    }
    footholds
}

fn load_tiles(
    map: &crate::wz::Node,
    base: &crate::wz::Node,
    load_context: &mut LoadContext<'_>,
) -> Vec<WzSpriteData> {
    let mut tiles = Vec::new();

    for i in 0..8u8 {
        let layer = match map.at_path(&i.to_string()) {
            Ok(l) => l,
            Err(_) => continue,
        };

        let Some(tile_set) = layer.get_opt::<String>("info/tS") else { continue; };

        if let Ok(tile_root) = layer.at_path("tile") {
            let mut children = tile_root.children();
            children.sort_by(|x1, _x2, x3, _x4| {
                x1.as_str().parse::<i32>().unwrap()
                    .cmp(&x3.as_str().parse::<i32>().unwrap())
            });
            for (name, tile_node) in children {
                let variant: String = tile_node.required("u");
                let index: i32 = tile_node.required("no");
                let (x, y) = tile_node.read_pos().unwrap();
                let z_m: i32 = tile_node.get_or("zM", 0);
                let tile_id: i32 = name.as_str().parse().unwrap_or(0);

                let img_path = format!("Map/Tile/{}.img/{}/{}", tile_set, variant, index);
                let img_node = base.at_path(&img_path).unwrap();
                let z: i32 = img_node.get_or("z", 0);
                let origin = img_node.get_vec2_opt("origin").unwrap();

                let handle = load_or_decode_image(&img_node, load_context, img_path);
                tiles.push(WzSpriteData {
                    image: handle, x, y, z, z_m, layer: i,
                    zid: tile_id, origin,
                });
            }
        }
    }

    tiles
}

fn load_objs(
    map: &crate::wz::Node,
    base: &crate::wz::Node,
    load_context: &mut LoadContext<'_>,
) -> Vec<WzSpriteData> {
    let mut objs = Vec::new();

    for i in 0..8u8 {
        let layer = match map.at_path(&i.to_string()) {
            Ok(l) => l,
            Err(_) => continue,
        };

        if let Ok(obj_root) = layer.at_path("obj") {
            for (name, obj_node) in obj_root.children() {
                let obj_set: String = obj_node.required("oS");
                let layer0: String = obj_node.required("l0");
                let layer1: String = obj_node.required("l1");
                let layer2: String = obj_node.required("l2");
                let (x, y) = obj_node.read_pos().unwrap();
                let z: i32 = obj_node.get_or("z", 0);
                let z_m: i32 = obj_node.get_or("zM", 0);
                let zid: i32 = name.as_str().parse().unwrap_or(0);

                let img_path = format!("Map/Obj/{}.img/{}/{}/{}/0", obj_set, layer0, layer1, layer2);
                let img_node = base.at_path(&img_path).unwrap();
                let origin = img_node.get_vec2_opt("origin").unwrap();

                let handle = load_or_decode_image(&img_node, load_context, img_path);
                objs.push(WzSpriteData {
                    image: handle, x, y, z, z_m, layer: i,
                    zid, origin,
                });
            }
        }
    }

    objs
}

fn load_backgrounds(
    map: &crate::wz::Node,
    base: &crate::wz::Node,
    load_context: &mut LoadContext<'_>,
) -> Vec<BackgroundData> {
    let back_root = match map.at_path("back") {
        Ok(n) => n,
        Err(_) => return Vec::new(),
    };

    let mut children = back_root.children();
    children.sort_by(|a, _, b, _| {
        a.as_str().parse::<i32>().unwrap_or(0)
            .cmp(&b.as_str().parse::<i32>().unwrap_or(0))
    });

    let mut backgrounds = Vec::new();
    for (name, back_node) in children {
        let index: i32 = name.as_str().parse().unwrap_or(0);
        let b_s: String = match back_node.get_opt::<String>("bS") {
            Some(v) => v,
            None => continue,
        };
        let no: i32 = back_node.get_or("no", 0);
        let front: bool = back_node.get_or::<bool>("front", false);
        let rx: i32 = back_node.get_or("rx", 100);
        let ry: i32 = back_node.get_or("ry", 100);
        let btype: i32 = back_node.get_or("type", 0);
        let cx: i32 = back_node.get_or("cx", 0);
        let cy: i32 = back_node.get_or("cy", 0);
        let alpha: i32 = back_node.get_or("a", 255);
        let (x, y) = back_node.read_pos().unwrap_or((0.0, 0.0));

        let img_path = format!("Map/Back/{}.img/back/{}", b_s, no);
        let img_node = match base.at_path(&img_path) {
            Ok(n) => n,
            Err(_) => continue,
        };

        let frame_node = if img_node.has("0") && TryInto::<DynamicImage>::try_into(img_node.clone()).is_err() {
            img_node.at_path("0").unwrap()
        } else {
            img_node
        };

        let img_label = format!("{}/0", img_path);
        let handle = load_or_decode_image(&frame_node, load_context, img_label);

        let origin = frame_node.get_vec2_opt("origin").unwrap_or_default();

        backgrounds.push(BackgroundData {
            image: handle, front, rx, ry, btype, cx, cy,
            alpha: alpha.clamp(0, 255) as u8,
            x, y, origin, index,
        });
    }

    backgrounds
}

fn load_life(map: &crate::wz::Node) -> Vec<LifeSpawn> {
    let life_root = match map.at_path("life") {
        Ok(n) => n,
        Err(_) => return Vec::new(),
    };

    let mut life = Vec::new();
    for (_name, life_node) in life_root.children() {
        let spawn_type: String = match life_node.get_opt::<String>("type") {
            Some(v) => v,
            None => continue,
        };
        let id: i32 = life_node.get_or("id", 0);
        let (x, y) = life_node.read_pos().unwrap_or((0.0, 0.0));
        let cy: i32 = life_node.get_or("cy", 0);
        let fh: i32 = life_node.get_or("fh", 0);
        let rx0: i32 = life_node.get_or("rx0", 0);
        let rx1: i32 = life_node.get_or("rx1", 0);
        let mob_time: i32 = life_node.get_or("mobTime", 0);
        let hide: bool = life_node.get_or::<bool>("hide", false);
        let flip: bool = life_node.get_or::<bool>("f", false);

        life.push(LifeSpawn { spawn_type, id, x, y, cy, fh, rx0, rx1, mob_time, hide, flip });
    }

    life
}

fn load_portals(map: &crate::wz::Node) -> Vec<PortalData> {
    let portal_root = match map.at_path("portal") {
        Ok(n) => n,
        Err(_) => return Vec::new(),
    };

    let mut portals = Vec::new();
    for (_name, portal_node) in portal_root.children() {
        let pt: i32 = portal_node.get_or("pt", 0);
        let pn: String = portal_node.get_or("pn", String::new());
        let (x, y) = portal_node.read_pos().unwrap_or((0.0, 0.0));
        let tm: i32 = portal_node.get_or("tm", 0);
        let tn: String = portal_node.get_or("tn", String::new());
        let script: Option<String> = portal_node.get_opt("script");
        let delay: Option<i32> = portal_node.get_opt("delay");
        let horizontal_impact: Option<i32> = portal_node.get_opt("horizontalImpact");
        let vertical_impact: Option<i32> = portal_node.get_opt("verticalImpact");
        let only_once: Option<i32> = portal_node.get_opt("onlyOnce");

        portals.push(PortalData { pt, pn, x, y, tm, tn, script, delay, horizontal_impact, vertical_impact, only_once });
    }

    portals
}

fn load_ladder_ropes(map: &crate::wz::Node) -> Vec<LadderRopeData> {
    let lr_root = match map.at_path("ladderRope") {
        Ok(n) => n,
        Err(_) => return Vec::new(),
    };

    let mut lrs = Vec::new();
    for (_name, lr_node) in lr_root.children() {
        let x: f32 = lr_node.get_or("x", 0.0);
        let raw_y1: f32 = lr_node.get_or("y1", 0.0);
        let raw_y2: f32 = lr_node.get_or("y2", 0.0);
        let y1 = -raw_y1;
        let y2 = -raw_y2;
        let is_ladder: bool = lr_node.get_or::<i32>("l", 0) == 0;
        let uf: i32 = lr_node.get_or("uf", 0);
        let page: i32 = lr_node.get_or("page", 0);

        lrs.push(LadderRopeData { x, y1, y2, is_ladder, uf, page });
    }

    lrs
}

fn load_seats(map: &crate::wz::Node) -> Vec<SeatData> {
    let seat_root = match map.at_path("seat") {
        Ok(n) => n,
        Err(_) => return Vec::new(),
    };

    let mut seats = Vec::new();
    for (_name, seat_node) in seat_root.children() {
        let (x, y) = seat_node.read_pos().unwrap_or((0.0, 0.0));
        seats.push(SeatData { x, y });
    }

    seats
}

fn load_areas(map: &crate::wz::Node) -> Vec<AreaData> {
    let area_root = match map.at_path("area") {
        Ok(n) => n,
        Err(_) => return Vec::new(),
    };

    let mut areas = Vec::new();
    for (_name, area_node) in area_root.children() {
        let x1: i32 = area_node.get_or("x1", 0);
        let raw_y1: i32 = area_node.get_or("y1", 0);
        let x2: i32 = area_node.get_or("x2", 0);
        let raw_y2: i32 = area_node.get_or("y2", 0);
        areas.push(AreaData { x1, y1: -raw_y1, x2, y2: -raw_y2 });
    }

    areas
}

fn load_minimap(
    map: &crate::wz::Node,
    _base: &crate::wz::Node,
    load_context: &mut LoadContext<'_>,
) -> Option<MiniMapData> {
    let mm_node = map.at_path("miniMap").ok()?;

    let canvas_node = mm_node.at_path("canvas").ok()?;
    let handle = load_or_decode_image(&canvas_node, load_context, format!("{}/miniMap/canvas", map.path()));

    let x: Option<i32> = mm_node.get_opt("x");
    let y: Option<i32> = mm_node.get_opt::<i32>("y").map(|v| -v);
    let width: Option<i32> = mm_node.get_opt("width");
    let height: Option<i32> = mm_node.get_opt("height");
    let mag: Option<i32> = mm_node.get_opt("mag");

    Some(MiniMapData { image: handle, x, y, width, height, mag })
}

fn load_or_decode_image(
    node: &crate::wz::Node,
    load_context: &mut LoadContext<'_>,
    wz_path: String,
) -> Handle<Image> {
    if load_context.has_labeled_asset(&wz_path) {
        return load_context.get_label_handle::<Image>(&wz_path);
    }
    let dynamic_image: DynamicImage = node.clone().try_into().unwrap();
    let image = Image::new(
        Extent3d {
            width: dynamic_image.width(),
            height: dynamic_image.height(),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        dynamic_image.into_bytes(),
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    load_context.add_labeled_asset(wz_path, image)
}
