use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use image::DynamicImage;
use std::collections::HashMap;

use crate::character::skills::*;
use crate::character::loader::WzSpriteCache;
use crate::wz::Node;

fn load_effect_frames(
    base: &Node,
    skill_node: &Node,
    sub_path: &str,
    cache: &mut WzSpriteCache,
    images: &mut Assets<Image>,
) -> Vec<EffectFrame> {
    let Ok(eff_node) = skill_node.at_path(sub_path) else {
        return Vec::new();
    };
    let mut frames = Vec::new();
    let children = eff_node.children();
    let mut keys: Vec<u32> = children
        .keys()
        .filter_map(|k| k.to_string().parse::<u32>().ok())
        .collect();
    keys.sort();

    for key in keys {
        let Ok(frame_node) = eff_node.at_path(&key.to_string()) else {
            continue;
        };
        let path = frame_node.path();
        let image = cache.get_or_load(&frame_node, &path, images);

        let origin = frame_node
            .at_path("origin")
            .ok()
            .and_then(|n| {
                let v: Result<crate::wz::Vector2D, _> = n.try_into();
                v.ok()
            })
            .map(|v| Vec2::new(v.0 as f32, v.1 as f32))
            .unwrap_or(Vec2::ZERO);

        let z: i32 = frame_node
            .at_path("z")
            .ok()
            .and_then(|n| n.try_into().ok())
            .unwrap_or(0);

        let delay: u32 = frame_node
            .at_path("delay")
            .ok()
            .and_then(|n| n.try_into().ok())
            .unwrap_or(100);

        let alpha0: Option<u8> = frame_node
            .at_path("a0")
            .ok()
            .and_then(|n| n.try_into().ok());

        let alpha1: Option<u8> = frame_node
            .at_path("a1")
            .ok()
            .and_then(|n| n.try_into().ok());

        frames.push(EffectFrame {
            image,
            origin,
            z,
            delay,
            alpha0,
            alpha1,
        });
    }

    frames
}

fn load_skill_icon(
    base: &Node,
    skill_node: &Node,
    icon_name: &str,
    cache: &mut WzSpriteCache,
    images: &mut Assets<Image>,
) -> Handle<Image> {
    if let Ok(icon_node) = skill_node.at_path(icon_name) {
        let path = icon_node.path();
        return cache.get_or_load(&icon_node, &path, images);
    }
    images.add(Image::new(
        Extent3d {
            width: 32,
            height: 32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0u8; 32 * 32 * 4],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ))
}

fn load_skill_levels(skill_node: &Node) -> HashMap<u32, SkillLevelData> {
    let Ok(level_node) = skill_node.at_path("level") else {
        return HashMap::new();
    };
    let mut levels = HashMap::new();
    for (key, _) in level_node.children() {
        let Ok(lvl) = key.to_string().parse::<u32>() else {
            continue;
        };
        let Ok(lvl_node) = level_node.at_path(&lvl.to_string()) else {
            continue;
        };
        levels.insert(
            lvl,
            SkillLevelData {
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
            },
        );
    }
    levels
}

fn load_reqs(skill_node: &Node) -> HashMap<u32, u32> {
    let Ok(req_node) = skill_node.at_path("req") else {
        return HashMap::new();
    };
    let mut reqs = HashMap::new();
    for (key, _) in req_node.children() {
        let Ok(skill_id) = key.to_string().parse::<u32>() else {
            continue;
        };
        let level: Result<i32, _> = req_node.at_path(&skill_id.to_string())?.try_into();
        if let Ok(lvl) = level {
            reqs.insert(skill_id, lvl as u32);
        }
    }
    reqs
}

pub fn load_skill_database(
    base: &Node,
    cache: &mut WzSpriteCache,
    images: &mut Assets<Image>,
) -> SkillDatabase {
    let mut skills = HashMap::new();

    let Ok(skill_root) = base.at_path("Skill") else {
        return SkillDatabase { skills };
    };

    for (class_name, _) in skill_root.children() {
        let class_name = class_name.to_string();
        if !class_name.ends_with(".img") {
            continue;
        }
        let Ok(class_node) = skill_root.at_path(&class_name) else {
            continue;
        };
        let Ok(skill_dir) = class_node.at_path("skill") else {
            continue;
        };

        for (skill_id_str, _) in skill_dir.children() {
            let Ok(skill_id) = skill_id_str.to_string().parse::<u32>() else {
                continue;
            };
            let Ok(skill_node) = skill_dir.at_path(&skill_id.to_string()) else {
                continue;
            };

            let skill_type_raw: Option<i32> = skill_node.at_path("skillType").ok().and_then(|n| n.try_into().ok());
            let skill_type = skill_type_raw.map(SkillType::from_raw).unwrap_or(SkillType::Active);

            let name: String = base
                .at_path(&format!("String/Skill.img/{}/name", skill_id))
                .ok()
                .and_then(|n| n.try_into().ok())
                .unwrap_or_default();

            let desc: String = base
                .at_path(&format!("String/Skill.img/{}/desc", skill_id))
                .ok()
                .and_then(|n| n.try_into().ok())
                .unwrap_or_default();

            let icon = load_skill_icon(base, &skill_node, "icon", cache, images);
            let icon_disabled = load_skill_icon(base, &skill_node, "iconDisabled", cache, images);
            let icon_mouse_over = load_skill_icon(base, &skill_node, "iconMouseOver", cache, images);

            let action: Option<String> = skill_node
                .at_path("action/0")
                .ok()
                .and_then(|n| n.try_into().ok());

            let prepare_action: Option<String> = skill_node
                .at_path("prepare/action")
                .ok()
                .and_then(|n| n.try_into().ok());

            let effect_frames = load_effect_frames(base, &skill_node, "effect", cache, images);
            let hit_frames = load_effect_frames(base, &skill_node, "hit", cache, images);
            let keydown_frames = load_effect_frames(base, &skill_node, "keydown", cache, images);

            let levels = load_skill_levels(&skill_node);
            let req = load_reqs(&skill_node);

            let master_level: u32 = skill_node
                .at_path("masterLevel")
                .ok()
                .and_then(|n| n.try_into().ok())
                .unwrap_or(0);

            let invisible: bool = skill_node
                .at_path("invisible")
                .ok()
                .and_then(|n| n.try_into().ok())
                .unwrap_or(false);

            skills.insert(
                skill_id,
                SkillEntry {
                    id: skill_id,
                    skill_type,
                    name,
                    desc,
                    levels,
                    action,
                    prepare_action,
                    effect_frames,
                    hit_frames,
                    keydown_frames,
                    icon,
                    icon_disabled,
                    icon_mouse_over,
                    req,
                    master_level,
                    invisible,
                    skill_type_raw,
                },
            );
        }
    }

    SkillDatabase { skills }
}
