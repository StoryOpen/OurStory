use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use image::DynamicImage;
use std::collections::HashMap;

use crate::character::types::*;
use crate::wz::Node;
use crate::wz::Vector2D;

#[derive(Resource, Default)]
pub struct WzSpriteCache {
    pub handles: HashMap<String, Handle<Image>>,
}

impl WzSpriteCache {
    pub fn get_or_load(
        &mut self,
        node: &Node,
        wz_path: &str,
        images: &mut Assets<Image>,
    ) -> Handle<Image> {
        if let Some(handle) = self.handles.get(wz_path) {
            return handle.clone();
        }
        let dynamic_image: DynamicImage = match node.extract_image() {
            Ok(img) => img,
            Err(_) => {
                // Node is not an image (e.g. a container with sub-nodes)
                warn!(
                    "get_or_load: extract_image failed for '{}', using 1x1 placeholder",
                    wz_path
                );
                let handle = images.add(Image::new(
                    Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    vec![0u8; 4],
                    TextureFormat::Rgba8Unorm,
                    RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                ));
                self.handles.insert(wz_path.to_string(), handle.clone());
                return handle;
            }
        };
        trace!(
            "get_or_load: loaded image {}x{} from '{}'",
            dynamic_image.width(),
            dynamic_image.height(),
            wz_path
        );
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
        let handle = images.add(image);
        self.handles.insert(wz_path.to_string(), handle.clone());
        handle
    }
}

fn load_part(
    node: &Node,
    part_name: &str,
    source: PartSource,
    zmap: &ZMap,
    slot_map: &SlotMap,
    cache: &mut WzSpriteCache,
    images: &mut Assets<Image>,
) -> Option<SpriteLayer> {
    let part_node = match node.at_path(part_name).ok() {
        Some(n) => n,
        None => {
            debug!("load_part: at_path('{}') failed in node '{}'", part_name, node.path());
            return None;
        }
    };
    let origin_node = match part_node.try_get("origin") {
        Some(n) => n,
        None => {
            debug!("load_part: no 'origin' in '{}' (part: {})", part_node.path(), part_name);
            return None;
        }
    };
    let origin_v = match origin_node.read_origin(&part_node).ok() {
        Some(v) => v,
        None => {
            debug!("load_part: read_origin failed in '{}' (part: {})", part_node.path(), part_name);
            return None;
        }
    };
    let origin = Vec2::new(origin_v.0 as f32, origin_v.1 as f32);
    let z_str: String = match part_node.at_path("z").ok().and_then(|n| n.try_into().ok()) {
        Some(s) => s,
        None => {
            debug!("load_part: no valid 'z' in '{}' (part: {})", part_node.path(), part_name);
            return None;
        }
    };
    let z = zmap.depth(&z_str);
    let path = part_node.path();
    let image = cache.get_or_load(&part_node, &path, images);

    let mut map = HashMap::new();
    if let Ok(map_node) = part_node.at_path("map") {
        for (child_name, _) in map_node.children() {
            if let Some(val) = map_node
                .at_path(child_name.as_str())
                .ok()
                .and_then(|n| {
                    let v: Vector2D = n.try_into().ok()?;
                    Some(Vec2::new(v.0 as f32, -(v.1 as f32)))
                })
            {
                map.insert(child_name.to_string(), val);
            }
        }
    }

    Some(SpriteLayer {
        image,
        origin,
        map,
        z,
        layer_name: part_name.to_string(),
        slot: slot_map.slot_for(&z_str).map(String::from),
        source,
    })
}

fn load_frame(
    base: &Node,
    body_action_path: &str,
    action_name: &str,
    head_action_path: Option<&str>,
    equip_configs: &[(EquipSlot, u32)],
    hair_id: Option<u32>,
    zmap: &ZMap,
    slot_map: &SlotMap,
    frame_idx: u32,
    cache: &mut WzSpriteCache,
    images: &mut Assets<Image>,
) -> Option<FrameData> {
    let frame_path = format!("{}/{}", body_action_path, frame_idx);
    let frame_node = base.at_path(&frame_path).ok()?;

    let delay: i32 = frame_node
        .at_path("delay")
        .ok()
        .and_then(|n| n.try_into().ok())
        .unwrap_or(100);

    // Check if this is a frame-reference action (references another action's frame)
    if let (Ok(action_node), Ok(frame_node_val)) = (
        frame_node.at_path("action"),
        frame_node.at_path("frame"),
    ) {
        let ref_action: String = action_node.try_into().ok()?;
        let ref_frame: i32 = frame_node_val.try_into().ok()?;
        let ref_body_action_path = body_action_path
            .rsplit_once('/')
            .map(|(parent, _)| format!("{}/{}", parent, ref_action))
            .unwrap_or_else(|| format!("{}/{}", body_action_path, ref_action));
        let ref_head_path = head_action_path
            .as_ref()
            .and_then(|h| {
                h.rsplit_once('/')
                    .map(|(parent, _)| format!("{}/{}", parent, ref_action))
            });

        let mut frame = load_frame(
            base,
            &ref_body_action_path,
            &ref_action,
            ref_head_path.as_deref(),
            equip_configs,
            hair_id,
            zmap,
            slot_map,
            ref_frame as u32,
            cache,
            images,
        )?;
        // Frame-reference delay overrides the referenced frame's delay
        frame.delay = delay.unsigned_abs();
        return Some(frame);
    }

    let mut parts = Vec::new();

    // Body frame parts in deterministic order (body is the anchor)
    for child_name in ["body", "arm", "lHand", "rHand"] {
        if let Some(layer) = load_part(
            &frame_node,
            child_name,
            PartSource::Body,
            zmap,
            slot_map,
            cache,
            images,
        ) {
            parts.push(layer);
        }
    }

    if let Some(head_path) = head_action_path {
        let head_frame_path = format!("{}/{}", head_path, frame_idx);
        if let Ok(head_frame) = base.at_path(&head_frame_path) {
            if let Some(layer) = load_part(
                &head_frame,
                "head",
                PartSource::Head,
                zmap,
                slot_map,
                cache,
                images,
            ) {
                parts.push(layer);
            }
        }
    }

    if let Some(hid) = hair_id {
        let hair_path = format!(
            "Character/Hair/{:08}.img/{}/{}",
            hid, action_name, frame_idx
        );
        if let Ok(hair_node) = base.at_path(&hair_path) {
            for (part_name, _) in hair_node.children() {
                let part_name = part_name.as_str();
                if let Some(layer) = load_part(
                    &hair_node,
                    part_name,
                    PartSource::Hair,
                    zmap,
                    slot_map,
                    cache,
                    images,
                ) {
                    parts.push(layer);
                }
            }
        }
    }

    for (slot, item_id) in equip_configs {
        let item_path = format!("Character/{}/{:08}.img", slot.dir_name(), item_id);
        let item_action_path = format!("{}/{}/{}", item_path, action_name, frame_idx);
        match base.at_path(&item_action_path) {
            Ok(item_frame) => {
                let mut loaded = 0u32;
                for part_name in slot.part_names() {
                    if let Some(layer) = load_part(
                        &item_frame,
                        part_name,
                        PartSource::Equipment(*slot),
                        zmap,
                        slot_map,
                        cache,
                        images,
                    ) {
                        parts.push(layer);
                        loaded += 1;
                    }
                }
                debug!(
                    "load_frame: equip {} ({:08}) action '{}' frame {}: {} part(s) loaded",
                    slot.dir_name(), item_id, action_name, frame_idx, loaded
                );
            }
            Err(_) => {
                trace!(
                    "load_frame: equip path not found: {}",
                    item_action_path
                );
            }
        }
    }

    Some(FrameData {
        parts,
        delay: delay.unsigned_abs(),
    })
}

fn preload_face_expressions(
    base: &Node,
    face_id: u32,
    zmap: &ZMap,
    slot_map: &SlotMap,
    cache: &mut WzSpriteCache,
    images: &mut Assets<Image>,
) -> HashMap<String, Vec<FrameData>> {
    let face_path = format!("Character/Face/{:08}.img", face_id);
    let face_root = match base.at_path(&face_path) {
        Ok(n) => n,
        Err(_) => return HashMap::new(),
    };

    let mut result = HashMap::new();

    for (expr_name, _) in face_root.children() {
        let expr_name = String::from(expr_name);
        if expr_name == "info" {
            continue;
        }

        let expr_node = match face_root.at_path(&expr_name) {
            Ok(n) => n,
            Err(_) => continue,
        };

        let child_keys: Vec<String> = expr_node
            .children()
            .into_iter()
            .map(|(n, _)| n.to_string())
            .collect();

        let mut frames = Vec::new();

        if child_keys.iter().any(|k| k == "face") {
            if let Some(layer) = load_part(
                &expr_node,
                "face",
                PartSource::Face,
                zmap,
                slot_map,
                cache,
                images,
            ) {
                frames.push(FrameData {
                    parts: vec![layer],
                    delay: 2000,
                });
            }
        } else if child_keys.iter().any(|k| k.parse::<u32>().is_ok()) {
            for key in &child_keys {
                if let Ok(idx) = key.parse::<u32>() {
                    if let Ok(frame_node) = expr_node.at_path(&idx.to_string()) {
                        if let Ok(delay_node) = frame_node.at_path("delay") {
                            let delay: Result<i32, _> = delay_node.try_into();
                            if let Ok(delay) = delay {
                                if let Some(layer) = load_part(
                                    &frame_node,
                                    "face",
                                    PartSource::Face,
                                    zmap,
                                    slot_map,
                                    cache,
                                    images,
                                ) {
                                    frames.push(FrameData {
                                        parts: vec![layer],
                                        delay: delay as u32,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        if !frames.is_empty() {
            result.insert(expr_name, frames);
        }
    }

    result
}

pub struct LoadedCharacterData {
    pub actions: HashMap<String, Vec<FrameData>>,
    pub face_expressions: HashMap<String, Vec<FrameData>>,
}

pub fn preload_character_frames(
    base: &Node,
    skin_suffix: u32,
    hair_id: Option<u32>,
    face_id: Option<u32>,
    equipment: &[(EquipSlot, u32)],
    zmap: &ZMap,
    slot_map: &SlotMap,
    cache: &mut WzSpriteCache,
    images: &mut Assets<Image>,
) -> LoadedCharacterData {
    let body_path = format!("Character/0000{:04}.img", skin_suffix);
    let head_path = format!("Character/0001{:04}.img", skin_suffix);

    let body_root = match base.at_path(&body_path) {
        Ok(n) => n,
        Err(_) => {
            warn!("body frame not found: {}", body_path);
            return LoadedCharacterData {
                actions: HashMap::new(),
                face_expressions: HashMap::new(),
            };
        }
    };

    let head_root = base.at_path(&head_path).ok();
    let mut actions = HashMap::new();

    info!(
        "preload_character_frames: skin={}, hair={:?}, face={:?}, equipment_count={}",
        skin_suffix,
        hair_id,
        face_id,
        equipment.len(),
    );
    for (slot, item_id) in equipment {
        info!(
            "  equip config: {} {:08}",
            slot.dir_name(),
            item_id
        );
    }

    for (action_name, _) in body_root.children() {
        let action_name = String::from(action_name);
        if action_name == "info" {
            continue;
        }

        let action_node = match body_root.at_path(&action_name) {
            Ok(n) => n,
            Err(_) => continue,
        };

        let frame_count = action_node.children().len();
        if frame_count == 0 {
            continue;
        }

        let mut frames = Vec::new();
        let body_action_path = format!("{}/{}", body_path, action_name);
        let head_action_path = head_root
            .as_ref()
            .map(|_| format!("{}/{}", head_path, action_name));

        for frame_idx in 0..frame_count as u32 {
            if let Some(frame_data) = load_frame(
                base,
                &body_action_path,
                &action_name,
                head_action_path.as_deref(),
                equipment,
                hair_id,
                zmap,
                slot_map,
                frame_idx,
                cache,
                images,
            ) {
                frames.push(frame_data);
            }
        }

        let total_parts: usize = frames.iter().map(|f| f.parts.len()).sum();
        let frame_count = frames.len();
        if !frames.is_empty() {
            actions.insert(action_name.clone(), frames);
        }
        debug!(
            "preload: action '{}' -> {} frames, {} total parts",
            action_name,
            frame_count,
            total_parts,
        );
    }

    let face_expressions = face_id
        .map(|fid| preload_face_expressions(base, fid, zmap, slot_map, cache, images))
        .unwrap_or_default();

    LoadedCharacterData {
        actions,
        face_expressions,
    }
}
