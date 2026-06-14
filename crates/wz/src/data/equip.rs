use log::warn;
use std::collections::HashMap;
use crate::error::WzError;
use crate::node::Node;
use crate::vector2d::Vector2D;
use crate::data::common::{FrameData, SpriteLayerData, PartSource};

#[derive(Debug, Clone)]
pub struct EquipData {
    pub id: i32,
    pub slot: EquipSlot,
    pub info: EquipInfo,
    pub actions: HashMap<String, Vec<FrameData>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Default)]
pub struct EquipInfo {
    pub cash: bool,
    pub islot: Option<String>,
    pub vslot: Option<String>,
    pub req_level: Option<i32>,
    pub req_job: Option<i32>,
    pub req_str: Option<i32>,
    pub req_dex: Option<i32>,
    pub req_int: Option<i32>,
    pub req_luk: Option<i32>,
    pub attack: Option<i32>,
    pub attack_speed: Option<i32>,
    pub inc_pad: Option<i32>,
    pub price: Option<i32>,
    pub tuc: Option<i32>,
    pub after_image: Option<String>,
    pub sfx: Option<String>,
    pub stand: Option<i32>,
    pub walk: Option<i32>,
    pub icon_path: String,
    pub icon_raw_path: String,
}

impl EquipData {
    pub(crate) fn load(base: &Node, item_id: i32) -> Result<Self, WzError> {
        let slot = categorize_item(item_id);
        let dir = slot.dir_name();
        let wz_path = format!("Character/{dir}/{item_id:08}.img");
        let item_node = base.at_path(&wz_path)?;

        let info = load_equip_info(&item_node);
        let actions = load_equip_actions(base, &wz_path, &item_node)?;

        Ok(EquipData { id: item_id, slot, info, actions })
    }
}

fn categorize_item(item_id: i32) -> EquipSlot {
    let category = item_id / 10000;
    match category {
        100 => EquipSlot::Cap,
        101 => EquipSlot::Accessory,
        102 => EquipSlot::Accessory,
        103 => EquipSlot::Cape,
        104 => EquipSlot::Coat,
        105 => EquipSlot::Longcoat,
        106 => EquipSlot::Pants,
        107 => EquipSlot::Shoes,
        108 => EquipSlot::Glove,
        109 => EquipSlot::Shield,
        110 => EquipSlot::Cape,
        111 => EquipSlot::Ring,
        130..=199 => EquipSlot::Weapon,
        _ => EquipSlot::Weapon,
    }
}

fn load_equip_info(item_node: &Node) -> EquipInfo {
    let info_node = match item_node.at_path("info") {
        Ok(n) => n,
        Err(_) => return EquipInfo::default(),
    };

    let icon_path = info_node.at_path("icon").ok().map(|n| n.path()).unwrap_or_else(|| {
        warn!("load_equip_info: icon path missing, using default");
        String::new()
    });
    let icon_raw_path = info_node.at_path("iconRaw").ok().map(|n| n.path()).unwrap_or_else(|| {
        warn!("load_equip_info: iconRaw path missing, using default");
        String::new()
    });

    EquipInfo {
        cash: info_node.get_opt::<i32>("cash").unwrap_or_else(|| {
            warn!("load_equip_info: cash missing, using 0");
            0
        }) != 0,
        islot: info_node.get_opt("islot"),
        vslot: info_node.get_opt("vslot"),
        req_level: info_node.get_opt("reqLevel"),
        req_job: info_node.get_opt("reqJob"),
        req_str: info_node.get_opt("reqSTR"),
        req_dex: info_node.get_opt("reqDEX"),
        req_int: info_node.get_opt("reqINT"),
        req_luk: info_node.get_opt("reqLUK"),
        attack: info_node.get_opt("attack"),
        attack_speed: info_node.get_opt("attackSpeed"),
        inc_pad: info_node.get_opt("incPAD"),
        price: info_node.get_opt("price"),
        tuc: info_node.get_opt("tuc"),
        after_image: info_node.get_opt("afterImage"),
        sfx: info_node.get_opt("sfx"),
        stand: info_node.get_opt("stand"),
        walk: info_node.get_opt("walk"),
        icon_path,
        icon_raw_path,
    }
}

fn load_equip_actions(base: &Node, wz_path: &str, item_node: &Node) -> Result<HashMap<String, Vec<FrameData>>, WzError> {
    let mut actions = HashMap::new();

    for (action_name, _) in item_node.children() {
        let action_name = String::from(action_name);
        if action_name == "info" { continue; }

        let action_node = match item_node.at_path(&action_name) {
            Ok(n) => n,
            Err(_) => continue,
        };

        let frame_count = action_node.children().len();
        if frame_count == 0 { continue; }

        let mut frames = Vec::new();
        for frame_idx in 0..frame_count as u32 {
            let frame_path = format!("{}/{}/{}", wz_path, action_name, frame_idx);
            let frame_node = match base.at_path(&frame_path) {
                Ok(n) => n,
                Err(_) => continue,
            };

            let delay: i32 = frame_node.at_path("delay").ok()
                .and_then(|n| n.try_into().ok())
                .unwrap_or_else(|| {
                    warn!("load_equip_actions: '{}' missing delay, using 100", frame_path);
                    100
                });

            let mut parts = Vec::new();
            for (child_name, _) in frame_node.children() {
                if child_name.as_str() == "delay" { continue; }
                if let Some(layer) = load_equip_part(&frame_node, child_name.as_str()) {
                    parts.push(layer);
                }
            }

            if !parts.is_empty() {
                frames.push(FrameData { parts, delay: delay.unsigned_abs() });
            }
        }

        if !frames.is_empty() {
            actions.insert(action_name, frames);
        }
    }

    Ok(actions)
}

fn load_equip_part(frame_node: &Node, part_name: &str) -> Option<SpriteLayerData> {
    let part_node = frame_node.at_path(part_name).ok()?;
    let origin_node = part_node.try_get("origin")?;
    let origin = origin_node.read_origin(&part_node).ok()?;
    let z_str: String = part_node.at_path("z").ok().and_then(|n| n.try_into().ok())?;
    let image_path = part_node.path();

    let mut map = std::collections::HashMap::new();
    if let Ok(map_node) = part_node.at_path("map") {
        for (child_name, _) in map_node.children() {
            if let Some(val) = map_node
                .at_path(child_name.as_str()).ok()
                .and_then(|n| {
                    let v: Result<wz_reader::property::Vector2D, _> = n.try_into();
                    v.ok().map(|v| Vector2D(v.0 as f32, -(v.1 as f32)))
                })
            {
                map.insert(child_name.to_string(), val);
            }
        }
    }

    Some(SpriteLayerData {
        image_path,
        origin,
        map,
        z: 0.0,
        layer_name: part_name.to_string(),
        slot: Some(z_str),
        source: PartSource::Equipment,
    })
}
