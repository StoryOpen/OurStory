use std::collections::{BTreeMap, HashMap};
use crate::error::WzError;
use crate::node::Node;
use crate::vector2d::Vector2D;

#[derive(Debug, Clone)]
pub struct MobData {
    pub id: i32,
    pub name: String,
    pub info: MobInfo,
    pub actions: HashMap<String, MobAction>,
}

#[derive(Debug, Clone)]
pub struct MobInfo {
    pub level: i32,
    pub max_hp: i32,
    pub max_mp: i32,
    pub exp: i32,
    pub pad: i32,
    pub pdd: i32,
    pub mad: i32,
    pub mdd: i32,
    pub acc: i32,
    pub eva: i32,
    pub speed: i32,
    pub body_attack: i32,
    pub undead: bool,
    pub pushed: i32,
    pub mob_type: i32,
    pub summon_type: i32,
    pub elem_attr: Option<String>,
    pub fs: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct MobAction {
    pub frames: Vec<MobFrame>,
}

#[derive(Debug, Clone)]
pub struct MobFrame {
    pub delay: u32,
    pub parts: Vec<MobPart>,
}

#[derive(Debug, Clone)]
pub struct MobPart {
    pub name: String,
    pub image_path: String,
    pub origin: Vector2D,
}

impl MobData {
    pub(crate) fn load(base: &Node, id: i32) -> Result<Self, WzError> {
        let wz_path = format!("Mob/{:07}.img", id);
        let mob_node = base.at_path(&wz_path)?;

        let info = Self::load_info(base, id, &mob_node)?;
        let name = base.at_path(&format!("String/Mob.img/{id}/name"))
            .ok()
            .and_then(|n| -> Option<String> { n.try_into().ok() })
            .unwrap_or_default();

        let mut actions = HashMap::new();
        for (action_name, action_node) in mob_node.children() {
            let name = action_name.to_string();
            if name == "info" { continue; }
            if !action_node.try_get("0").is_some() { continue; }

            let mut frame_map: BTreeMap<u32, MobFrame> = BTreeMap::new();

            for (frame_key, frame_node) in action_node.children() {
                let frame_index = match frame_key.to_string().parse::<u32>() {
                    Ok(i) => i,
                    Err(_) => continue,
                };
                let delay = frame_node
                    .try_get("delay")
                    .and_then(|n| -> Option<u32> {
                        let v: i32 = n.try_into().ok()?;
                        Some(v.max(0) as u32)
                    })
                    .unwrap_or(100);

                let mut parts = Vec::new();
                let is_direct_sprite = frame_node.extract_image().is_ok();

                if is_direct_sprite {
                    let image_path = frame_node.path();
                    let origin = frame_node.try_get("origin")
                        .and_then(|n| n.read_origin(&frame_node).ok())
                        .unwrap_or(Vector2D::ZERO);
                    parts.push(MobPart { name: "body".to_string(), image_path, origin });
                } else {
                    for (part_name, part_node) in frame_node.children() {
                        let pn = part_name.to_string();
                        if pn == "delay" || pn == "face" || pn == "z" { continue; }
                        if !part_node.try_get("origin").is_some()
                            && !part_node.try_get("_inlink").is_some()
                            && !part_node.try_get("_outlink").is_some()
                        { continue; }

                        let origin = part_node.try_get("origin")
                            .and_then(|n| n.read_origin(&part_node).ok())
                            .unwrap_or(Vector2D::ZERO);
                        let image_path = part_node.path();

                        parts.push(MobPart { name: pn, image_path, origin });
                    }
                    sort_parts(&mut parts);
                }

                frame_map.insert(frame_index, MobFrame { delay, parts });
            }

            let frames: Vec<MobFrame> = frame_map.into_values().collect();
            if !frames.is_empty() {
                actions.insert(name, MobAction { frames });
            }
        }

        Ok(MobData { id, name, info, actions })
    }

    fn load_info(_base: &Node, _mob_id: i32, mob_node: &Node) -> Result<MobInfo, WzError> {
        let info = mob_node.at_path("info")?;

        fn read_int(node: &Node, path: &str) -> i32 {
            node.at_path(path).ok().and_then(|n| -> Option<i32> { n.try_into().ok() }).unwrap_or(0)
        }

        Ok(MobInfo {
            level: read_int(&info, "level"),
            max_hp: read_int(&info, "maxHP"),
            max_mp: read_int(&info, "maxMP"),
            exp: read_int(&info, "exp"),
            pad: read_int(&info, "PADamage"),
            pdd: read_int(&info, "PDDamage"),
            mad: read_int(&info, "MADamage"),
            mdd: read_int(&info, "MDDamage"),
            acc: read_int(&info, "acc"),
            eva: read_int(&info, "eva"),
            speed: read_int(&info, "speed"),
            body_attack: read_int(&info, "bodyAttack"),
            undead: read_int(&info, "undead") != 0,
            pushed: read_int(&info, "pushed"),
            mob_type: read_int(&info, "mobType"),
            summon_type: read_int(&info, "summonType"),
            elem_attr: info.get_opt("elemAttr"),
            fs: info.get_opt("fs"),
        })
    }
}

fn sort_parts(parts: &mut Vec<MobPart>) {
    const ORDER: &[&str] = &[
        "back", "body", "arm", "armOverHair", "head", "headOverHair",
        "face", "weapon", "cap", "cape", "glove", "shoes",
    ];
    parts.sort_by_key(|p| {
        ORDER.iter().position(|&k| k == p.name).unwrap_or(usize::MAX)
    });
}
