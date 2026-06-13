use std::collections::HashMap;
use crate::error::WzError;
use crate::node::Node;

#[derive(Debug, Clone)]
pub struct SkillDatabase {
    pub skills: HashMap<u32, SkillEntry>,
}

#[derive(Debug, Clone)]
pub struct SkillEntry {
    pub id: u32,
    pub skill_type: SkillType,
    pub name: String,
    pub desc: String,
    pub levels: HashMap<u32, SkillLevelData>,
    pub req: HashMap<u32, u32>,
    pub icon_path: String,
    pub icon_disabled_path: String,
    pub icon_mouse_over_path: String,
    pub action: Option<String>,
    pub prepare_action: Option<String>,
    pub effect_paths: Vec<String>,
    pub hit_paths: Vec<String>,
    pub keydown_paths: Vec<String>,
    pub master_level: u32,
    pub invisible: bool,
    pub skill_type_raw: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkillType {
    Passive,
    Active,
    AttackProc,
    Special,
}

impl SkillType {
    fn from_raw(raw: i32) -> Self {
        match raw {
            1 => SkillType::Passive,
            2 => SkillType::Active,
            3 => SkillType::AttackProc,
            4 => SkillType::Special,
            _ => SkillType::Active,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkillLevelData {
    pub damage: Option<i32>,
    pub mp_con: Option<i32>,
    pub hp_con: Option<i32>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub time: Option<i32>,
    pub prop: Option<i32>,
    pub pad: Option<i32>,
    pub mad: Option<i32>,
    pub pdd: Option<i32>,
    pub mdd: Option<i32>,
    pub acc: Option<i32>,
    pub eva: Option<i32>,
    pub speed: Option<i32>,
    pub jump: Option<i32>,
    pub hs: Option<String>,
}

impl SkillDatabase {
    pub(crate) fn load(base: &Node) -> Result<Self, WzError> {
        let mut skills = HashMap::new();

        let skill_root = match base.at_path("Skill") {
            Ok(n) => n,
            Err(_) => return Ok(SkillDatabase { skills }),
        };

        for (class_name, _) in skill_root.children() {
            let class_name = class_name.to_string();
            if !class_name.ends_with(".img") { continue; }
            let Ok(class_node) = skill_root.at_path(&class_name) else { continue; };
            let Ok(skill_dir) = class_node.at_path("skill") else { continue; };

            for (skill_id_str, _) in skill_dir.children() {
                let Ok(skill_id) = skill_id_str.to_string().parse::<u32>() else { continue; };
                let Ok(skill_node) = skill_dir.at_path(&skill_id.to_string()) else { continue; };

                let skill_type_raw: Option<i32> = skill_node.at_path("skillType").ok().and_then(|n| n.try_into().ok());
                let skill_type = skill_type_raw.map(SkillType::from_raw).unwrap_or(SkillType::Active);

                let name: String = base
                    .at_path(&format!("String/Skill.img/{skill_id}/name"))
                    .ok().and_then(|n| n.try_into().ok())
                    .unwrap_or_default();

                let desc: String = base
                    .at_path(&format!("String/Skill.img/{skill_id}/desc"))
                    .ok().and_then(|n| n.try_into().ok())
                    .unwrap_or_default();

                let icon_path = skill_node.at_path("icon").ok().map(|n| n.path()).unwrap_or_default();
                let icon_disabled_path = skill_node.at_path("iconDisabled").ok().map(|n| n.path()).unwrap_or_default();
                let icon_mouse_over_path = skill_node.at_path("iconMouseOver").ok().map(|n| n.path()).unwrap_or_default();

                let action: Option<String> = skill_node.at_path("action/0").ok().and_then(|n| n.try_into().ok());
                let prepare_action: Option<String> = skill_node.at_path("prepare/action").ok().and_then(|n| n.try_into().ok());

                let effect_paths = collect_frame_paths(&skill_node, "effect");
                let hit_paths = collect_frame_paths(&skill_node, "hit");
                let keydown_paths = collect_frame_paths(&skill_node, "keydown");

                let levels = load_skill_levels(&skill_node);
                let req = load_reqs(&skill_node);

                let master_level: u32 = skill_node
                    .at_path("masterLevel").ok()
                    .and_then(|n| -> Option<i32> { n.try_into().ok() })
                    .map(|v| v as u32)
                    .unwrap_or(0);

                let invisible: bool = skill_node
                    .at_path("invisible").ok()
                    .and_then(|n| n.try_into().ok())
                    .unwrap_or(false);

                skills.insert(skill_id, SkillEntry {
                    id: skill_id,
                    skill_type,
                    name,
                    desc,
                    levels,
                    action,
                    prepare_action,
                    effect_paths,
                    hit_paths,
                    keydown_paths,
                    icon_path,
                    icon_disabled_path,
                    icon_mouse_over_path,
                    req,
                    master_level,
                    invisible,
                    skill_type_raw,
                });
            }
        }

        Ok(SkillDatabase { skills })
    }
}

fn collect_frame_paths(skill_node: &Node, sub_path: &str) -> Vec<String> {
    let Ok(eff_node) = skill_node.at_path(sub_path) else { return Vec::new() };
    collect_paths_recursive(&eff_node)
}

fn collect_paths_recursive(node: &Node) -> Vec<String> {
    let children = node.children();
    let mut keys: Vec<u32> = children.keys()
        .filter_map(|k| k.to_string().parse::<u32>().ok())
        .collect();
    keys.sort();

    let mut paths = Vec::new();
    for key in keys {
        let Ok(child) = node.at_path(&key.to_string()) else { continue; };
        if child.extract_image().is_ok() {
            paths.push(child.path());
        } else {
            paths.extend(collect_paths_recursive(&child));
        }
    }
    paths
}

fn load_skill_levels(skill_node: &Node) -> HashMap<u32, SkillLevelData> {
    let Ok(level_node) = skill_node.at_path("level") else { return HashMap::new() };
    let mut levels = HashMap::new();
    for (key, _) in level_node.children() {
        let Ok(lvl) = key.to_string().parse::<u32>() else { continue; };
        let Ok(lvl_node) = level_node.at_path(&lvl.to_string()) else { continue; };
        levels.insert(lvl, SkillLevelData {
            damage: lvl_node.at_path("damage").ok().and_then(|n| n.try_into().ok()),
            mp_con: lvl_node.at_path("mpCon").ok().and_then(|n| n.try_into().ok()),
            hp_con: lvl_node.at_path("hpCon").ok().and_then(|n| n.try_into().ok()),
            x: lvl_node.at_path("x").ok().and_then(|n| n.try_into().ok()),
            y: lvl_node.at_path("y").ok().and_then(|n| n.try_into().ok()),
            time: lvl_node.at_path("time").ok().and_then(|n| n.try_into().ok()),
            prop: lvl_node.at_path("prop").ok().and_then(|n| n.try_into().ok()),
            pad: lvl_node.at_path("pad").ok().and_then(|n| n.try_into().ok()),
            mad: lvl_node.at_path("mad").ok().and_then(|n| n.try_into().ok()),
            pdd: lvl_node.at_path("pdd").ok().and_then(|n| n.try_into().ok()),
            mdd: lvl_node.at_path("mdd").ok().and_then(|n| n.try_into().ok()),
            acc: lvl_node.at_path("acc").ok().and_then(|n| n.try_into().ok()),
            eva: lvl_node.at_path("eva").ok().and_then(|n| n.try_into().ok()),
            speed: lvl_node.at_path("speed").ok().and_then(|n| n.try_into().ok()),
            jump: lvl_node.at_path("jump").ok().and_then(|n| n.try_into().ok()),
            hs: lvl_node.at_path("hs").ok().and_then(|n| n.try_into().ok()),
        });
    }
    levels
}

fn load_reqs(skill_node: &Node) -> HashMap<u32, u32> {
    let Ok(req_node) = skill_node.at_path("req") else { return HashMap::new() };
    let mut reqs = HashMap::new();
    for (key, _) in req_node.children() {
        let Ok(skill_id) = key.to_string().parse::<u32>() else { continue; };
        let level: Result<i32, _> = req_node.at_path(&skill_id.to_string()).and_then(|n| n.try_into());
        if let Ok(lvl) = level {
            reqs.insert(skill_id, lvl as u32);
        }
    }
    reqs
}
