use std::collections::{BTreeMap, HashMap};

use bevy::{
    asset::{AssetLoader, LoadContext, RenderAssetUsages, io::Reader},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use image::DynamicImage;
use thiserror::Error;
use wz_reader::WzNodeCast;

use crate::wz::{Node, Vector2D};

#[derive(Asset, TypePath, Debug)]
pub struct WzMobAsset {
    pub info: MobInfo,
    pub actions: HashMap<String, MobAction>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct MobInfo {
    pub mob_id: i32,
    pub name: String,
    pub level: i32,
    pub max_hp: i32,
    pub max_mp: i32,
    pub exp: i32,
    pub pad: i32,
    pub pdd: i32,
    pub speed: i32,
    pub undead: bool,
    pub body_attack: i32,
}

#[derive(Debug)]
pub struct MobAction {
    pub frames: Vec<MobFrame>,
}

#[derive(Debug)]
pub struct MobFrame {
    pub delay: u32,
    pub parts: Vec<MobPart>,
}

#[derive(Debug)]
pub struct MobPart {
    pub name: String,
    pub image_handle: Handle<Image>,
    pub origin: Vec2,
}

#[derive(Debug, Error)]
pub enum MobLoaderError {
    #[error("WZ node error: {0}")]
    WzError(#[from] crate::wz::NodeError),
}

#[derive(Default, TypePath)]
pub struct WzMobLoader;

impl AssetLoader for WzMobLoader {
    type Asset = WzMobAsset;
    type Settings = ();
    type Error = MobLoaderError;

    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let asset_path = load_context.path().path().to_string_lossy().to_string();
        let wz_path = asset_path
            .strip_suffix(".mob")
            .unwrap_or(&asset_path)
            .to_string();

        let base = crate::wz::get_cached_base();
        let mob_node = base.at_path(&wz_path)?;

        let mob_id = parse_mob_id(&wz_path);
        let info = parse_mob_info(&mob_node, mob_id)?;

        let mut actions = HashMap::new();
        for (action_name, action_node) in mob_node.children() {
            let name = action_name.to_string();
            if name == "info" {
                continue;
            }
            if !action_node.try_get("0").is_some() {
                continue;
            }

            let n = &action_node;
            let mut frame_map: BTreeMap<u32, MobFrame> = BTreeMap::new();

            for (frame_key, frame_node) in n.children() {
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

                let is_direct_sprite = frame_node
                    .wz_node
                    .read()
                    .is_ok_and(|g| g.try_as_png().is_some());

                if is_direct_sprite {
                    let label = format!("{wz_path}/{name}/{frame_index}");
                    let image_handle = load_or_decode_image(&frame_node, load_context, label);
                    let origin = frame_node
                        .try_get("origin")
                        .and_then(|n| -> Option<Vector2D> { n.try_into().ok() })
                        .map(|v| Vec2::new(v.0 as f32, v.1 as f32))
                        .unwrap_or(Vec2::ZERO);
                    parts.push(MobPart {
                        name: "body".to_string(),
                        image_handle,
                        origin,
                    });
                } else {
                    for (part_name, part_node) in frame_node.children() {
                        let pn = part_name.to_string();
                        if pn == "delay" || pn == "face" || pn == "z" {
                            continue;
                        }

                        if !part_node.try_get("origin").is_some()
                            && !part_node.try_get("_inlink").is_some()
                            && !part_node.try_get("_outlink").is_some()
                        {
                            continue;
                        }

                        let origin = part_node
                            .try_get("origin")
                            .and_then(|n| -> Option<Vector2D> { n.try_into().ok() })
                            .map(|v| Vec2::new(v.0 as f32, v.1 as f32))
                            .unwrap_or(Vec2::ZERO);

                        let label = format!("{wz_path}/{name}/{frame_index}/{pn}");
                        let image_handle = load_or_decode_image(&part_node, load_context, label);

                        parts.push(MobPart {
                            name: pn,
                            image_handle,
                            origin,
                        });
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

        Ok(WzMobAsset { info, actions })
    }

    fn extensions(&self) -> &[&str] {
        &["mob"]
    }
}

fn parse_mob_id(wz_path: &str) -> i32 {
    wz_path
        .trim_end_matches(".img")
        .rsplit('/')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

fn parse_mob_info(mob_node: &Node, mob_id: i32) -> Result<MobInfo, MobLoaderError> {
    let info = mob_node.at_path("info")?;

    let base = crate::wz::get_cached_base();
    let name = base
        .at_path(&format!("String/Mob.img/{mob_id}/name"))
        .ok()
        .and_then(|n| -> Option<String> { n.try_into().ok() })
        .unwrap_or_default();

    fn read_int(info: &Node, path: &str) -> i32 {
        info.at_path(path)
            .ok()
            .and_then(|n| -> Option<i32> { n.try_into().ok() })
            .unwrap_or(0)
    }

    Ok(MobInfo {
        mob_id,
        name,
        level: read_int(&info, "level"),
        max_hp: read_int(&info, "maxHP"),
        max_mp: read_int(&info, "maxMP"),
        exp: read_int(&info, "exp"),
        pad: read_int(&info, "PADamage"),
        pdd: read_int(&info, "PDDamage"),
        speed: read_int(&info, "speed"),
        undead: read_int(&info, "undead") != 0,
        body_attack: read_int(&info, "bodyAttack"),
    })
}

fn sort_parts(parts: &mut Vec<MobPart>) {
    const ORDER: &[&str] = &[
        "back",
        "body",
        "arm",
        "armOverHair",
        "head",
        "headOverHair",
        "face",
        "weapon",
        "cap",
        "cape",
        "glove",
        "shoes",
    ];

    parts.sort_by_key(|p| {
        ORDER
            .iter()
            .position(|&k| k == p.name)
            .unwrap_or(usize::MAX)
    });
}

fn load_or_decode_image(
    node: &Node,
    load_context: &mut LoadContext<'_>,
    label: String,
) -> Handle<Image> {
    if load_context.has_labeled_asset(&label) {
        return load_context.get_label_handle::<Image>(&label);
    }

    let dynamic_image = extract_image(node);
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
    load_context.add_labeled_asset(label, image)
}

fn extract_image(node: &Node) -> DynamicImage {
    if let Some(inlink_node) = node.try_get("_inlink") {
        let path: String = inlink_node.try_into().expect("_inlink not a string");
        let resolved = resolve_img_relative(node, &path).expect("failed to resolve _inlink target");
        return extract_image(&resolved);
    }

    if let Some(outlink_node) = node.try_get("_outlink") {
        let path: String = outlink_node.try_into().expect("_outlink not a string");
        let base = crate::wz::get_cached_base();
        let resolved = base
            .at_path(&path)
            .expect("failed to resolve _outlink target");
        return extract_image(&resolved);
    }

    match wz_reader::property::png::get_image(&node.wz_node) {
        Ok(img) => img,
        Err(e) => panic!("extract_image failed at {}: {:?}", node.path(), e),
    }
}

/// Resolve a path relative to the current node (handles `..`).
fn resolve_img_relative(node: &Node, rel_path: &str) -> Option<Node> {
    let current_path = node.path();
    let mut segs: Vec<&str> = current_path.split('/').collect();
    segs.pop();

    for part in rel_path.split('/') {
        match part {
            ".." => {
                segs.pop();
            }
            "." => {}
            _ => segs.push(part),
        }
    }

    let absolute = segs.join("/");
    crate::wz::get_cached_base().at_path(&absolute).ok()
}
