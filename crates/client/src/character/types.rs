use bevy::prelude::*;
use std::collections::HashMap;
use wz_reader::WzNodeCast;

#[derive(Resource)]
pub struct ZMap {
    pub layers: HashMap<String, usize>,
}

const ZMAP_MAX: usize = 150;

impl ZMap {
    /// Returns a relative depth offset (0..=150). Callers should add a layer
    /// base z (e.g. `GameLayer::Character.base_z()`) to get the final z.
    pub fn depth(&self, z: &str) -> f32 {
        let index = self.layers.get(z).copied().unwrap_or(ZMAP_MAX);
        (ZMAP_MAX - index) as f32
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

#[derive(Resource)]
pub struct SlotMap {
    pub layers: HashMap<String, String>,
}

impl SlotMap {
    pub fn slot_for(&self, z: &str) -> Option<&str> {
        self.layers.get(z).map(String::as_str)
    }

    /// Splits a compound slot code (e.g. "MaGw", "CpHdH1H2...") into its
    /// individual 2-character slot codes. Returns empty if `z` is unknown.
    pub fn slots_for(&self, z: &str) -> Vec<&str> {
        match self.layers.get(z) {
            Some(s) if s.len() % 2 == 0 => s
                .as_bytes()
                .chunks(2)
                .filter_map(|c| std::str::from_utf8(c).ok())
                .collect(),
            _ => Vec::new(),
        }
    }
}

pub fn load_smap(base: &crate::wz::Node) -> SlotMap {
    let mut layers = HashMap::new();
    if let Ok(smap_node) = base.at_path("smap.img") {
        let guard = smap_node.wz_node.read().expect("lock poisoned");
        if let Some(image) = guard.try_as_image() {
            if let Ok((children, _)) = image.resolve_children(None) {
                for (name, child) in children.into_iter() {
                    let s = child
                        .read()
                        .ok()
                        .and_then(|g| g.try_as_string().and_then(|s| s.get_string().ok()));
                    if let Some(s) = s {
                        layers.insert(name.to_string(), s);
                    }
                }
            }
        }
    }
    SlotMap { layers }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
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

    /// Canonical 2-char smap slot code for this equipment slot. Used to
    /// cross-check that a part loaded from this slot's WZ directory was
    /// actually authored against the same z-layer ownership the game expects.
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

#[derive(Debug, Clone)]
pub struct SpriteLayer {
    pub image: Handle<Image>,
    pub origin: Vec2,
    pub map: HashMap<String, Vec2>,
    pub z: f32,
    pub layer_name: String,
    /// Compound smap slot code resolved from the part's `z` value. `None` if
    /// the z-layer is not in the smap (e.g. body, head, hair — body-part
    /// layers that aren't slot-routed).
    pub slot: Option<String>,
    /// Which loaded source produced this part. Used by the vslot-suppression
    /// filter to exclude the part's own source from suppressing itself.
    pub source: PartSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartSource {
    Body,
    Head,
    Hair,
    Face,
    Equipment(EquipSlot),
}

#[derive(Debug, Clone)]
pub struct FrameData {
    pub parts: Vec<SpriteLayer>,
    pub delay: u32,
    pub flip: bool,
}

#[derive(Debug, Clone)]
pub struct EquipmentEntry {
    pub slot: EquipSlot,
    pub item_id: u32,
    /// 2-char slot codes from `info/vslot`, split into individual codes.
    pub vslot: Vec<String>,
}

/// Splits a vslot string (e.g. "CpHdH1H2H3H4H5H6HsHfHbAfAyAsAe") into
/// individual 2-char codes. Returns empty if `s` is empty or odd-length.
pub fn split_vslot(s: &str) -> Vec<String> {
    if s.len() % 2 != 0 {
        return Vec::new();
    }
    s.as_bytes()
        .chunks(2)
        .filter_map(|c| std::str::from_utf8(c).ok().map(String::from))
        .collect()
}

/// Reads `info/vslot` from an item .img node and returns the 2-char codes.
/// Returns an empty vec if the property is missing.
pub fn load_vslot(node: &crate::wz::Node) -> Vec<String> {
    let Ok(vslot_node) = node.at_path("info/vslot") else {
        return Vec::new();
    };
    let Ok(s) = TryInto::<String>::try_into(vslot_node) else {
        return Vec::new();
    };
    split_vslot(&s)
}

/// Splits a SpriteLayer's smap compound (`slot` field) into 2-char codes.
fn part_codes(part: &SpriteLayer) -> Vec<&str> {
    match &part.slot {
        Some(s) if s.len() % 2 == 0 => s
            .as_bytes()
            .chunks(2)
            .filter_map(|c| std::str::from_utf8(c).ok())
            .collect(),
        _ => Vec::new(),
    }
}

/// Returns true if the part's source is the given equipment slot. Used to
/// exclude a part's own source from suppressing itself.
fn sourced_from(part: &SpriteLayer, slot: EquipSlot) -> bool {
    matches!(part.source, PartSource::Equipment(s) if s == slot)
}

/// Filters out sprites that are hidden by an equipped item's vslot.
///
/// Rule: for each part, look up the 2-char codes from its smap compound.
/// If any equipped item (other than the part's own source) has that code in
/// its vslot, the part is suppressed. Items with deeper zmap sprites win
/// over items with shallower sprites when both claim the same code, but
/// because vslot is the authoritative declaration of "I cover this code",
/// any claim from another item is treated as suppression.
///
/// Parts with no smap entry (e.g. some body/hair sub-layers not in smap) are
/// always kept.
pub fn filter_hidden_sprites(
    parts: Vec<SpriteLayer>,
    equipment: &[EquipmentEntry],
) -> Vec<SpriteLayer> {
    parts
        .into_iter()
        .filter(|part| {
            let codes = part_codes(part);
            if codes.is_empty() {
                return true;
            }
            for entry in equipment {
                if sourced_from(part, entry.slot) {
                    continue;
                }
                if entry.vslot.iter().any(|c| codes.contains(&c.as_str())) {
                    return false;
                }
            }
            true
        })
        .collect()
}

fn compute_connection_point(part_local: Vec3, origin: Vec2, map_entry: Vec2) -> Vec2 {
    Vec2::new(
        part_local.x + origin.x + map_entry.x,
        part_local.y + origin.y + map_entry.y,
    )
}

/// Compute root-local transforms for all parts in a frame using hierarchical
/// connection-point matching. Parts with `navel` attach to root center;
/// parts without `navel` attach to the first matching named connection point
/// from already-positioned parts.
///
/// `z_base` is added to each part's relative z offset (from `SpriteLayer.z`).
/// Pass `GameLayer::Character.base_z()` for game-semantic layering.
pub fn compute_frame_transforms(parts: &[SpriteLayer], z_base: f32) -> HashMap<String, Vec3> {
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
                target.y - part.origin.y - map_entry.y,
                z_base + part.z,
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
