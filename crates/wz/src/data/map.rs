use log::warn;
use crate::error::WzError;
use crate::node::Node;
use crate::vector2d::Vector2D;
use crate::data::common::{AnimFrame, Foothold};

#[derive(Debug, Clone)]
pub struct MapData {
    pub id: i32,
    pub name: String,
    pub street_name: String,
    pub info: MapInfo,
    pub layers: Vec<MapLayer>,
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
    pub vr_left: Option<i32>,
    pub vr_right: Option<i32>,
    pub vr_top: Option<i32>,
    pub vr_bottom: Option<i32>,
    pub map_mark: Option<String>,
    pub no_map_cmd: Option<i32>,
    pub swim: Option<i32>,
    pub version: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct MapLayer {
    pub tiles: Vec<TilePlacement>,
    pub objs: Vec<ObjPlacement>,
}

#[derive(Debug, Clone)]
pub struct TilePlacement {
    pub image_path: String,
    pub pos: Vector2D,
    pub z: i32,
    pub zid: i32,
    pub origin: Vector2D,
    pub animation_frames: Vec<AnimFrame>,
}

#[derive(Debug, Clone)]
pub struct ObjPlacement {
    pub image_path: String,
    pub pos: Vector2D,
    pub z: i32,
    pub zid: i32,
    pub origin: Vector2D,
    pub animation_frames: Vec<AnimFrame>,
    pub flip: bool,
    pub flow: i32,
    pub rx: i32,
    pub ry: i32,
    pub cx: i32,
    pub cy: i32,
}

#[derive(Debug, Clone)]
pub struct BackgroundData {
    pub image_path: String,
    pub front: bool,
    pub rx: i32,
    pub ry: i32,
    pub btype: i32,
    pub cx: i32,
    pub cy: i32,
    pub alpha: u8,
    pub flip: bool,
    pub pos: Vector2D,
    pub origin: Vector2D,
    pub index: i32,
    pub animation_frames: Vec<AnimFrame>,
}

#[derive(Debug, Clone)]
pub struct LifeSpawn {
    pub spawn_type: String,
    pub id: i32,
    pub pos: Vector2D,
    pub cy: i32,
    pub fh: i32,
    pub rx0: i32,
    pub rx1: i32,
    pub mob_time: i32,
    pub hide: bool,
    pub flip: bool,
}

#[derive(Debug, Clone)]
pub struct PortalData {
    pub pt: i32,
    pub pn: String,
    pub pos: Vector2D,
    pub tm: i32,
    pub tn: String,
    pub script: Option<String>,
    pub delay: Option<i32>,
    pub horizontal_impact: Option<i32>,
    pub vertical_impact: Option<i32>,
    pub only_once: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct LadderRopeData {
    pub x: f32,
    pub y1: f32,
    pub y2: f32,
    pub is_ladder: bool,
    pub uf: i32,
    pub page: i32,
}

#[derive(Debug, Clone)]
pub struct SeatData {
    pub pos: Vector2D,
}

#[derive(Debug, Clone)]
pub struct AreaData {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

#[derive(Debug, Clone)]
pub struct MiniMapData {
    pub image_path: String,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub mag: Option<i32>,
}

impl MapData {
    pub(crate) fn load(base: &Node, id: i32) -> Result<Self, WzError> {
        let region = id / 100000000;
        let wz_path = format!("Map/Map/Map{region}/{id}.img");
        let map = base.at_path(&wz_path)?;

        let name = base.at_path(&format!("String/Map.img/{id}/mapName"))
            .ok().and_then(|n| -> Option<String> { n.try_into().ok() })
            .unwrap_or_else(|| {
                warn!("Map {id}: mapName not found, using default");
                String::new()
            });
        let street_name = base.at_path(&format!("String/Map.img/{id}/streetName"))
            .ok().and_then(|n| -> Option<String> { n.try_into().ok() })
            .unwrap_or_else(|| {
                warn!("Map {id}: streetName not found, using default");
                String::new()
            });

        let info = load_info(&map);
        let footholds = load_footholds(&map);
        let layers = load_layers(&map, base);
        let backgrounds = load_backgrounds(&map, base);
        let life = load_life(&map);
        let portals = load_portals(&map);
        let ladder_ropes = load_ladder_ropes(&map);
        let seats = load_seats(&map);
        let areas = load_areas(&map);
        let minimap = load_minimap(&map);

        Ok(MapData {
            id,
            name,
            street_name,
            info,
            layers,
            footholds,
            backgrounds,
            life,
            portals,
            ladder_ropes,
            seats,
            areas,
            minimap,
        })
    }
}

fn load_info(map: &Node) -> MapInfo {
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
        vr_left: i("VRLeft"),
        vr_right: i("VRRight"),
        vr_top: i("VRTop"),
        vr_bottom: i("VRBottom"),
        map_mark: s("mapMark"),
        no_map_cmd: i("noMapCmd"),
        swim: i("swim"),
        version: i("version"),
    }
}

fn load_footholds(map: &Node) -> Vec<Foothold> {
    let mut footholds = Vec::new();
    if let Ok(fh_root) = map.at_path("foothold") {
        for (layer_name, group_node) in fh_root.children() {
            let layer_num: u8 = layer_name.as_str().parse().unwrap_or_else(|e| {
                warn!("load_footholds: non-numeric layer name '{}': {e}, using 0", layer_name.as_str());
                0
            });
            for (group_name, id_node) in group_node.children() {
                let group_num: i32 = group_name.as_str().parse().unwrap_or_else(|e| {
                    warn!("load_footholds: non-numeric group name '{}': {e}, using 0", group_name.as_str());
                    0
                });
                for (id_name, fh) in id_node.children() {
                    let Vector2D(x1, y1) = fh.read_pos_n(1).unwrap_or_else(|_| {
                        warn!("load_footholds: foothold missing pos1, using ZERO");
                        Vector2D::ZERO
                    });
                    let Vector2D(x2, y2) = fh.read_pos_n(2).unwrap_or_else(|_| {
                        warn!("load_footholds: foothold missing pos2, using ZERO");
                        Vector2D::ZERO
                    });
                    let id: i32 = id_name.as_str().parse().unwrap_or_else(|e| {
                        warn!("load_footholds: non-numeric foothold id '{}': {e}, using 0", id_name.as_str());
                        0
                    });
                    let force: i32 = fh.get_or("force", 0);
                    let forbid_fall: Option<i32> = fh.get_opt("forbidFall");
                    let piece: Option<i32> = fh.get_opt("piece");
                    let next_id: Option<i32> = fh.get_opt("next");
                    let prev_id: Option<i32> = fh.get_opt("prev");
                    let cant_through: bool = fh.get_or("cantThrough", false);
                    let forbid_fall_down: bool = fh.get_or("forbidFallDown", false);
                    footholds.push(Foothold {
                        id,
                        group: group_num,
                        layer: layer_num,
                        x1, y1, x2, y2,
                        force, forbid_fall, piece, next_id, prev_id,
                        cant_through, forbid_fall_down,
                    });
                }
            }
        }
    }
    footholds
}

fn load_layers(map: &Node, base: &Node) -> Vec<MapLayer> {
    let mut layers = Vec::new();
    for i in 0..8u8 {
        let layer = match map.at_path(&i.to_string()) {
            Ok(l) => l,
            Err(_) => { layers.push(MapLayer { tiles: Vec::new(), objs: Vec::new() }); continue; }
        };

        let tiles = load_tiles(&layer, base);
        let objs = load_objs(&layer, base);
        layers.push(MapLayer { tiles, objs });
    }
    layers
}

fn load_tiles(layer: &Node, base: &Node) -> Vec<TilePlacement> {
    let mut tiles = Vec::new();
    let Some(tile_set) = layer.get_opt::<String>("info/tS") else { return tiles; };

    if let Ok(tile_root) = layer.at_path("tile") {
        let mut children = tile_root.children();
        children.sort_by(|x1, _x2, x3, _x4| {
            let a = x1.as_str().parse::<i32>().unwrap_or_else(|e| {
                warn!("load_tiles: non-numeric tile key '{}': {e}, using 0", x1.as_str());
                0
            });
            let b = x3.as_str().parse::<i32>().unwrap_or_else(|e| {
                warn!("load_tiles: non-numeric tile key '{}': {e}, using 0", x3.as_str());
                0
            });
            a.cmp(&b)
        });
        for (name, tile_node) in children.iter() {
            let variant: String = tile_node.required("u");
            let index: i32 = tile_node.required("no");
            let tile_id: i32 = name.as_str().parse().unwrap_or_else(|e| {
                warn!("load_tiles: non-numeric tile name '{}': {e}, using 0", name.as_str());
                0
            });

            let img_path = format!("Map/Tile/{}.img/{}/{}", tile_set, variant, index);
            let Ok(img_node) = base.at_path(&img_path) else { continue; };
            let z: i32 = img_node.get_or("z", 0);
            let Ok(pos) = tile_node.read_pos() else { continue; };

            let (animation_frames, image_path, origin) = load_animated_node(&img_node, &img_path);

            tiles.push(TilePlacement {
                image_path,
                pos,
                z,
                zid: tile_id,
                origin,
                animation_frames,
            });
        }
    }

    tiles.sort_by_key(|t| (t.z, t.zid));
    tiles
}

fn load_objs(layer: &Node, base: &Node) -> Vec<ObjPlacement> {
    let mut objs = Vec::new();
    if let Ok(obj_root) = layer.at_path("obj") {
        for (name, obj_node) in obj_root.children() {
            let obj_set: String = obj_node.required("oS");
            let l0: String = obj_node.required("l0");
            let l1: String = obj_node.required("l1");
            let l2: String = obj_node.required("l2");
            let z: i32 = obj_node.get_or("z", 0);
            let zid: i32 = name.as_str().parse().unwrap_or_else(|e| {
                warn!("load_objs: non-numeric obj name '{}': {e}, using 0", name.as_str());
                0
            });
            let flip: bool = obj_node.get_or("f", false);
            let flow: i32 = obj_node.get_or("flow", 0);
            let rx: i32 = obj_node.get_or("rx", 0);
            let ry: i32 = obj_node.get_or("ry", 0);
            let mut cx: i32 = obj_node.get_or("cx", 0);
            let mut cy: i32 = obj_node.get_or("cy", 0);

            if flow & 1 != 0 && cx == 0 { cx = 1000; }
            if flow & 2 != 0 && cy == 0 { cy = 1000; }

            let img_path = format!("Map/Obj/{}.img/{}/{}/{}/0", obj_set, l0, l1, l2);
            let Ok(img_node) = base.at_path(&img_path) else { continue; };
            let Ok(pos) = obj_node.read_pos() else { continue; };

            let (animation_frames, image_path, origin) = load_animated_node(&img_node, &img_path);

            objs.push(ObjPlacement {
                image_path,
                pos,
                z,
                zid,
                origin,
                animation_frames,
                flip,
                flow,
                rx,
                ry,
                cx,
                cy,
            });
        }
    }

    objs.sort_by_key(|o| (o.z, o.zid));
    objs
}

fn load_backgrounds(map: &Node, base: &Node) -> Vec<BackgroundData> {
    let back_root = match map.at_path("back") {
        Ok(n) => n,
        Err(_) => return Vec::new(),
    };

    let mut backgrounds = Vec::new();
    for (name, back_node) in back_root.children() {
        let index: i32 = name.as_str().parse().unwrap_or_else(|e| {
            warn!("load_backgrounds: non-numeric bg key '{}': {e}, using 0", name.as_str());
            0
        });
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
        let flip: bool = back_node.get_or("f", false);

        let img_path = format!("Map/Back/{}.img/back/{}", b_s, no);
        let img_node = match base.at_path(&img_path) {
            Ok(n) => n,
            Err(_) => continue,
        };

        let pos = back_node.read_pos().unwrap_or_else(|_| {
            warn!("load_backgrounds: background missing position, using ZERO");
            Vector2D::ZERO
        });
        let (animation_frames, image_path, origin) = load_animated_node(&img_node, &img_path);

        backgrounds.push(BackgroundData {
            image_path,
            front,
            rx,
            ry,
            btype,
            cx,
            cy,
            alpha: alpha.clamp(0, 255) as u8,
            flip,
            pos,
            origin,
            index,
            animation_frames,
        });
    }

    backgrounds.sort_by_key(|b| b.index);
    backgrounds
}

fn load_life(map: &Node) -> Vec<LifeSpawn> {
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
        let id: i32 = life_node
            .get_opt::<String>("id")
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                warn!("load_life: non-numeric or missing id, using 0");
                0
            });
        let x: f32 = life_node.get_or("x", 0.0);
        let cy: i32 = life_node.get_or("cy", 0);
        let pos = Vector2D(x, -(cy as f32));
        let fh: i32 = life_node.get_or("fh", 0);
        let rx0: i32 = life_node.get_or("rx0", 0);
        let rx1: i32 = life_node.get_or("rx1", 0);
        let mob_time: i32 = life_node.get_or("mobTime", 0);
        let hide: bool = life_node.get_or::<bool>("hide", false);
        let flip: bool = life_node.get_or::<bool>("f", false);

        life.push(LifeSpawn { spawn_type, id, pos, cy, fh, rx0, rx1, mob_time, hide, flip });
    }

    life
}

fn load_portals(map: &Node) -> Vec<PortalData> {
    let portal_root = match map.at_path("portal") {
        Ok(n) => n,
        Err(_) => return Vec::new(),
    };

    let mut portals = Vec::new();
    for (_name, portal_node) in portal_root.children() {
        let pt: i32 = portal_node.get_or("pt", 0);
        let pn: String = portal_node.get_or("pn", String::new());
        let pos = portal_node.read_pos().unwrap_or_else(|_| {
            warn!("load_portals: portal '{}' missing position, using ZERO", pn);
            Vector2D::ZERO
        });
        let tm: i32 = portal_node.get_or("tm", 0);
        let tn: String = portal_node.get_or("tn", String::new());
        let script: Option<String> = portal_node.get_opt("script");
        let delay: Option<i32> = portal_node.get_opt("delay");
        let horizontal_impact: Option<i32> = portal_node.get_opt("horizontalImpact");
        let vertical_impact: Option<i32> = portal_node.get_opt("verticalImpact");
        let only_once: Option<i32> = portal_node.get_opt("onlyOnce");

        portals.push(PortalData { pt, pn, pos, tm, tn, script, delay, horizontal_impact, vertical_impact, only_once });
    }

    portals
}

fn load_ladder_ropes(map: &Node) -> Vec<LadderRopeData> {
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

fn load_seats(map: &Node) -> Vec<SeatData> {
    let seat_root = match map.at_path("seat") {
        Ok(n) => n,
        Err(_) => return Vec::new(),
    };

    let mut seats = Vec::new();
    for (_name, seat_node) in seat_root.children() {
        let pos = seat_node.read_pos().unwrap_or_else(|_| {
            warn!("load_seats: seat missing position, using ZERO");
            Vector2D::ZERO
        });
        seats.push(SeatData { pos });
    }

    seats
}

fn load_areas(map: &Node) -> Vec<AreaData> {
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

fn load_minimap(map: &Node) -> Option<MiniMapData> {
    let mm_node = map.at_path("miniMap").ok()?;
    let canvas_node = mm_node.at_path("canvas").ok()?;
    let image_path = canvas_node.path();

    let x: Option<i32> = mm_node.get_opt("x");
    let y: Option<i32> = mm_node.get_opt::<i32>("y").map(|v| -v);
    let width: Option<i32> = mm_node.get_opt("width");
    let height: Option<i32> = mm_node.get_opt("height");
    let mag: Option<i32> = mm_node.get_opt("mag");

    Some(MiniMapData { image_path, x, y, width, height, mag })
}

fn load_animated_node(node: &Node, _base_path: &str) -> (Vec<AnimFrame>, String, Vector2D) {
    let is_animated = node.has("0") && {
        #[cfg(feature = "image-data")]
        { TryInto::<image::DynamicImage>::try_into(node.clone()).is_err() }
        #[cfg(not(feature = "image-data"))]
        { false }
    };

    if !is_animated {
        let image_path = node.path();
        let origin = node.try_get("origin")
            .and_then(|n| n.read_origin(node).ok())
            .unwrap_or_else(|| {
                warn!("load_animated_node: node '{}' missing origin, using ZERO", node.path());
                Vector2D::ZERO
            });
        return (Vec::new(), image_path, origin);
    }

    let mut frames = Vec::new();
    let mut first_image_path = String::new();
    let mut first_origin = Vector2D::ZERO;

    let mut children = node.children();
    children.sort_by(|a, _, b, _| {
        let ai = a.as_str().parse::<i32>().unwrap_or_else(|e| {
            warn!("load_animated_node: non-numeric anim key '{}': {e}, using 0", a.as_str());
            0
        });
        let bi = b.as_str().parse::<i32>().unwrap_or_else(|e| {
            warn!("load_animated_node: non-numeric anim key '{}': {e}, using 0", b.as_str());
            0
        });
        ai.cmp(&bi)
    });

    for (name, child) in children {
        #[cfg(feature = "image-data")]
        {
            if name.as_str().parse::<i32>().ok().filter(|v| *v >= 0).is_none() { continue; }

            if TryInto::<image::DynamicImage>::try_into(child.clone()).is_err() { continue; }

            let image_path = child.path();
            let origin = child.try_get("origin")
                .and_then(|n| n.read_origin(&child).ok())
                .unwrap_or_else(|| {
                    warn!("load_animated_node: anim frame '{}' missing origin, using ZERO", child.path());
                    Vector2D::ZERO
                });
            let delay: i32 = child.get_or("delay", 100);
            let move_type: i32 = child.get_or("moveType", 0);
            let move_w: f32 = child.get_or("moveW", 0.0);
            let move_h: f32 = child.get_or("moveH", 0.0);
            let move_p: f32 = child.get_or("moveP", 6283.0);
            let move_r: f32 = child.get_or("moveR", 0.0);
            let a0: f32 = child.get_or("a0", 1.0);
            let a1: f32 = child.get_or("a1", 1.0);

            if first_image_path.is_empty() {
                first_image_path = image_path.clone();
                first_origin = origin;
            }

            frames.push(AnimFrame {
                image_path,
                origin,
                delay: delay as u32,
                move_type, move_w, move_h, move_p, move_r, a0, a1,
            });
        }
        #[cfg(not(feature = "image-data"))]
        { let _ = name; let _ = child; }
    }

    if first_image_path.is_empty() {
        first_image_path = node.path();
    }

    (frames, first_image_path, first_origin)
}
