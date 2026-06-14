use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource)]
pub struct ZMap {
    pub layers: HashMap<String, usize>,
}

const ZMAP_MAX: usize = 150;

impl ZMap {
    pub fn depth(&self, z: &str) -> f32 {
        let index = self.layers.get(z).copied().unwrap_or_else(|| {
            warn!("ZMap::depth: unknown z-layer '{}', using ZMAP_MAX ({})", z, ZMAP_MAX);
            ZMAP_MAX
        });
        (ZMAP_MAX - index) as f32
    }
}

pub fn load_zmap(wz: &wz::WzData) -> ZMap {
    let mut layers = HashMap::new();
    match wz.load_zmap() {
        Ok(entries) => {
            for (name, i) in entries {
                layers.insert(name, i);
            }
        }
        Err(e) => warn!("load_zmap: failed to load zmap: {e}, using empty ZMap"),
    }
    ZMap { layers }
}

#[derive(Resource)]
pub struct SlotMap {
    pub layers: HashMap<String, String>,
}

impl SlotMap {
    pub fn slot_for(&self, z: &str) -> Option<&str> {
        self.layers.get(z).map(String::as_str)
    }

    pub fn slots_for(&self, z: &str) -> Vec<&str> {
        match self.layers.get(z) {
            Some(s) if s.len() % 2 == 0 => s
                .as_bytes()
                .chunks(2)
                .filter_map(|c| match std::str::from_utf8(c) {
                    Ok(s) => Some(s),
                    Err(e) => {
                        warn!("slots_for: invalid UTF-8 chunk '{:?}': {e}", c);
                        None
                    }
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}

pub fn load_smap(wz: &wz::WzData) -> SlotMap {
    let mut layers = HashMap::new();
    match wz.load_smap() {
        Ok(entries) => {
            for (name, s) in entries {
                layers.insert(name, s);
            }
        }
        Err(e) => warn!("load_smap: failed to load smap: {e}, using empty SlotMap"),
    }
    SlotMap { layers }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Reflect)]
pub enum EquipSlot {
    Cap, Cape, Coat, Longcoat, Pants, Shoes, Glove, Weapon, Shield, Accessory, Ring,
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

    pub fn slot_code(&self) -> &'static str {
        match self {
            EquipSlot::Cap => "Cp",
            EquipSlot::Cape => "Sr",
            EquipSlot::Coat => "Ma",
            EquipSlot::Longcoat => "Ma",
            EquipSlot::Pants => "Pn",
            EquipSlot::Shoes => "So",
            EquipSlot::Glove => "Gl",
            EquipSlot::Weapon => "Wp",
            EquipSlot::Shield => "Si",
            EquipSlot::Accessory => "Af",
            EquipSlot::Ring => "Ri",
        }
    }
}

#[derive(Debug, Clone, Reflect)]
pub struct SpriteLayer {
    pub image: Handle<Image>,
    pub origin: Vec2,
    pub map: HashMap<String, Vec2>,
    pub z: f32,
    pub layer_name: String,
    pub slot: Option<String>,
    pub source: PartSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum PartSource {
    Body, Head, Hair, Face, Equipment(EquipSlot),
}

#[derive(Debug, Clone, Reflect)]
pub struct FrameData {
    pub parts: Vec<SpriteLayer>,
    pub delay: u32,
}

#[derive(Debug, Clone, Reflect)]
pub struct EquipmentEntry {
    pub slot: EquipSlot,
    pub item_id: u32,
    pub vslot: Vec<String>,
}

pub fn split_vslot(s: &str) -> Vec<String> {
    if s.len() % 2 != 0 { return Vec::new(); }
    s.as_bytes()
        .chunks(2)
        .filter_map(|c| match std::str::from_utf8(c) {
            Ok(s) => Some(String::from(s)),
            Err(e) => {
                warn!("split_vslot: invalid UTF-8 chunk '{:?}': {e}", c);
                None
            }
        })
        .collect()
}

pub fn load_vslot(wz: &wz::WzData, item_id: u32) -> Vec<String> {
    let equip = match wz.load_equip(item_id as i32) {
        Ok(e) => e,
        Err(e) => {
            warn!("load_vslot: failed to load equip {item_id}: {e}, returning empty");
            return Vec::new();
        }
    };
    equip.info.vslot.as_ref().map(|s| split_vslot(s)).unwrap_or_else(|| {
        warn!("load_vslot: equip {item_id} has no vslot, returning empty");
        Vec::new()
    })
}

fn slot_codes(part: &SpriteLayer) -> Vec<&str> {
    match &part.slot {
        Some(s) if s.len() % 2 == 0 => s
            .as_bytes()
            .chunks(2)
            .filter_map(|c| match std::str::from_utf8(c) {
                Ok(s) => Some(s),
                Err(e) => {
                    warn!("slot_codes: invalid UTF-8 chunk '{:?}': {e}", c);
                    None
                }
            })
            .collect(),
        _ => Vec::new(),
    }
}

pub fn filter_hidden_sprites(
    parts: Vec<SpriteLayer>,
    equipment: &[EquipmentEntry],
) -> Vec<SpriteLayer> {
    parts.into_iter().filter(|part| {
        match part.source {
            PartSource::Equipment(_) => true,
            _ => {
                let codes = slot_codes(part);
                if codes.is_empty() { return true; }
                !equipment.iter().any(|entry| {
                    codes.iter().all(|c| entry.vslot.iter().any(|v| v == *c))
                })
            }
        }
    }).collect()
}

pub fn compute_frame_transforms(
    parts: &[SpriteLayer],
    z_base: f32,
) -> (HashMap<String, Vec3>, HashMap<String, Option<String>>) {
    use std::collections::HashSet;
    let mut cpoints: HashMap<String, (String, Vec2)> = HashMap::new();
    let mut placed: HashSet<String> = HashSet::new();
    let mut transforms: HashMap<String, Vec3> = HashMap::new();
    let mut parents: HashMap<String, Option<String>> = HashMap::new();

    if let Some(body_part) = parts.iter().find(|p| p.layer_name == "body") {
        let p = Vec3::new(-body_part.origin.x, -body_part.origin.y, z_base + body_part.z);
        transforms.insert(body_part.layer_name.clone(), p);
        parents.insert(body_part.layer_name.clone(), None);
        placed.insert(body_part.layer_name.clone());
        for (name, val) in &body_part.map {
            let cpoint_local = Vec2::new(body_part.origin.x + val.x, body_part.origin.y + val.y);
            cpoints.insert(name.clone(), (body_part.layer_name.clone(), cpoint_local));
        }
    }

    while placed.len() < parts.len() {
        let mut placed_any = false;
        for part in parts {
            if placed.contains(&part.layer_name) { continue; }
            let mut found_attach: Option<(String, Vec2)> = None;
            for (key, val) in &part.map {
                if cpoints.contains_key(key) {
                    found_attach = Some((key.clone(), *val));
                    break;
                }
            }
            let pos = if let Some((cpoint_name, child_map_entry)) = found_attach {
                let (parent_name, parent_cpoint) = cpoints.get(&cpoint_name).expect("cpoint exists");
                let cpoint_local = Vec2::new(
                    transforms[parent_name].x + parent_cpoint.x,
                    transforms[parent_name].y + parent_cpoint.y,
                );
                Vec3::new(
                    cpoint_local.x - part.origin.x - child_map_entry.x,
                    cpoint_local.y - part.origin.y - child_map_entry.y,
                    z_base + part.z,
                )
            } else {
                Vec3::new(-part.origin.x, -part.origin.y, z_base + part.z)
            };
            transforms.insert(part.layer_name.clone(), pos);
            parents.insert(part.layer_name.clone(), None);
            placed.insert(part.layer_name.clone());
            placed_any = true;
            for (name, val) in &part.map {
                let cpoint_local = Vec2::new(part.origin.x + val.x, part.origin.y + val.y);
                cpoints.insert(name.clone(), (part.layer_name.clone(), cpoint_local));
            }
        }
        if !placed_any { break; }
    }
    (transforms, parents)
}
