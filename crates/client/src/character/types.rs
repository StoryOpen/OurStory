use std::collections::HashMap;
use bevy::prelude::*;
use wz_reader::WzNodeCast;

#[derive(Resource)]
pub struct ZMap {
    pub layers: HashMap<String, usize>,
}

const ZMAP_MAX: usize = 150;

impl ZMap {
    pub fn depth(&self, z: &str) -> f32 {
        let index = self.layers.get(z).copied().unwrap_or(ZMAP_MAX);
        (ZMAP_MAX - index) as f32 + 50.0
    }
}

pub fn load_zmap(base: &crate::wz::Node) -> ZMap {
    let mut layers = HashMap::new();
    if let Ok(zmap_node) = base.at_path("zmap.img") {
        let guard = zmap_node.wz_node.read().expect("lock poisoned");
        if let Some(image) = guard.try_as_image() {
            if let Ok((children, _)) = image.resolve_children(None) {
                for (i, (name, _)) in children.into_iter().enumerate() {
                    layers.insert(name.to_string(), i);
                }
            }
        }
    }
    ZMap { layers }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum EquipSlot {
    Cap,
    Cape,
    Coat,
    Longcoat,
    Pants,
    Shoes,
    Glove,
    Weapon,
    Shield,
    Accessory,
    Ring,
}

impl EquipSlot {
    pub fn dir_name(&self) -> &'static str {
        match self {
            EquipSlot::Cap => "Cap",
            EquipSlot::Cape => "Cape",
            EquipSlot::Coat => "Coat",
            EquipSlot::Longcoat => "Longcoat",
            EquipSlot::Pants => "Pants",
            EquipSlot::Shoes => "Shoes",
            EquipSlot::Glove => "Glove",
            EquipSlot::Weapon => "Weapon",
            EquipSlot::Shield => "Shield",
            EquipSlot::Accessory => "Accessory",
            EquipSlot::Ring => "Ring",
        }
    }

    pub fn part_names(&self) -> &'static [&'static str] {
        match self {
            EquipSlot::Cap => &["default", "backDefault"],
            EquipSlot::Cape => &["cape"],
            EquipSlot::Coat => &["mail", "mailArm"],
            EquipSlot::Longcoat => &["mail", "mailArm"],
            EquipSlot::Pants => &["pants"],
            EquipSlot::Shoes => &["shoes"],
            EquipSlot::Glove => &["rGlove", "lGlove"],
            EquipSlot::Weapon => &["weapon"],
            EquipSlot::Shield => &["shield"],
            EquipSlot::Accessory => &["accessory"],
            EquipSlot::Ring => &["ring"],
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpriteLayer {
    pub image: Handle<Image>,
    pub origin: Vec2,
    pub map: HashMap<String, Vec2>,
    pub z: f32,
    pub layer_name: String,
}

#[derive(Debug, Clone)]
pub struct FrameData {
    pub parts: Vec<SpriteLayer>,
    pub delay: u32,
}

fn compute_connection_point(
    part_local: Vec3,
    origin: Vec2,
    map_entry: Vec2,
) -> Vec2 {
    Vec2::new(
        part_local.x + origin.x + map_entry.x,
        part_local.y - origin.y - map_entry.y,
    )
}

/// Compute root-local transforms for all parts in a frame using hierarchical
/// connection-point matching. Parts with `navel` attach to root center;
/// parts without `navel` attach to the first matching named connection point
/// from already-positioned parts.
pub fn compute_frame_transforms(parts: &[SpriteLayer]) -> HashMap<String, Vec3> {
    use std::collections::{HashMap, HashSet};
    let mut cpoints: HashMap<String, Vec2> = HashMap::new();
    cpoints.insert("navel".into(), Vec2::ZERO);

    let mut placed: HashSet<String> = HashSet::new();
    let mut transforms: HashMap<String, Vec3> = HashMap::new();

    while placed.len() < parts.len() {
        let mut placed_any = false;

        for part in parts {
            if placed.contains(&part.layer_name) {
                continue;
            }

            let (attach_name, map_entry) = if let Some(navel) = part.map.get("navel") {
                ("navel".into(), *navel)
            } else {
                let mut found = None;
                for (key, val) in &part.map {
                    if cpoints.contains_key(key) {
                        found = Some((key.clone(), *val));
                        break;
                    }
                }
                match found {
                    Some(f) => f,
                    None => continue,
                }
            };

            let target = *cpoints.get(&attach_name).unwrap();
            let pos = Vec3::new(
                target.x - part.origin.x - map_entry.x,
                target.y + part.origin.y + map_entry.y,
                part.z,
            );

            transforms.insert(part.layer_name.clone(), pos);
            placed.insert(part.layer_name.clone());
            placed_any = true;

            for (name, offset) in &part.map {
                let cp = compute_connection_point(pos, part.origin, *offset);
                cpoints.insert(name.clone(), cp);
            }
        }

        if !placed_any {
            break;
        }
    }

    transforms
}
