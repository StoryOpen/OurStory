use bevy::prelude::*;
use std::collections::HashMap;

/// Z-ordering for character layers.
/// Loaded from WZ at startup via WzZMapAsset.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct ZMap {
    pub layers: HashMap<String, usize>,
}

const ZMAP_MAX: usize = 150;

impl ZMap {
    pub fn depth(&self, z: &str) -> f32 {
        let index = self.layers.get(z).copied().unwrap_or_else(|| {
            warn!("ZMap::depth: unknown z-layer '{}', using 0", z);
            0
        });
        (ZMAP_MAX - index) as f32
    }
}

/// Build a ZMap from the raw Vec returned by WzData::load_zmap().
pub fn zmap_from_entries(entries: Vec<(String, usize)>) -> ZMap {
    let mut layers = HashMap::new();
    for (name, i) in entries {
        layers.insert(name, i);
    }
    ZMap { layers }
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
