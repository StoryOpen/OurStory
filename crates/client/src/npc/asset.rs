use std::collections::{BTreeMap, HashMap};

use bevy::{
    asset::{AssetLoader, LoadContext, RenderAssetUsages, io::Reader},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use image::DynamicImage;
use thiserror::Error;

use crate::wz::Node;

#[derive(Asset, TypePath, Debug)]
pub struct WzNpcAsset {
    pub actions: HashMap<String, NpcAction>,
}

#[derive(Debug)]
pub struct NpcAction {
    pub frames: Vec<NpcFrame>,
}

#[derive(Debug)]
pub struct NpcFrame {
    pub delay: u32,
    pub sprite: NpcSprite,
}

#[derive(Debug)]
pub struct NpcSprite {
    pub image_handle: Handle<Image>,
    pub origin: Vec2,
}

#[derive(Debug, Error)]
pub enum NpcLoaderError {
    #[error("WZ node error: {0}")]
    WzError(#[from] crate::wz::NodeError),
}

#[derive(Default, TypePath)]
pub struct WzNpcLoader;

impl AssetLoader for WzNpcLoader {
    type Asset = WzNpcAsset;
    type Settings = ();
    type Error = NpcLoaderError;

    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let asset_path = load_context.path().path().to_string_lossy().to_string();
        let wz_path = asset_path
            .strip_suffix(".npc")
            .unwrap_or(&asset_path)
            .to_string();

        let base = crate::wz::get_cached_base();
        let npc_node = base.at_path(&wz_path)?;

        let mut actions = HashMap::new();
        for (action_name, action_node) in npc_node.children() {
            let name = action_name.to_string();
            if name == "info" {
                continue;
            }
            if !action_node.try_get("0").is_some() {
                continue;
            }

            let mut frame_map: BTreeMap<u32, NpcFrame> = BTreeMap::new();
            for (frame_key, frame_node) in action_node.children() {
                let frame_index: u32 = match frame_key.to_string().parse() {
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

                let label = format!("{wz_path}/{name}/{frame_index}");
                let image_handle = load_or_decode_image(&frame_node, load_context, label);
                let origin = frame_node
                    .try_get("origin")
                    .and_then(|n| n.read_origin(&frame_node).ok())
                    .map(|v| Vec2::new(v.0 as f32, v.1 as f32))
                    .unwrap_or_default();

                frame_map.insert(frame_index, NpcFrame {
                    delay,
                    sprite: NpcSprite {
                        image_handle,
                        origin,
                    },
                });
            }

            let frames: Vec<NpcFrame> = frame_map.into_values().collect();
            if !frames.is_empty() {
                actions.insert(name, NpcAction { frames });
            }
        }

        Ok(WzNpcAsset { actions })
    }

    fn extensions(&self) -> &[&str] {
        &["npc"]
    }
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
        let resolved =
            resolve_img_relative(node, &path).expect("failed to resolve _inlink target");
        return extract_image(&resolved);
    }

    if let Some(outlink_node) = node.try_get("_outlink") {
        let path: String = outlink_node.try_into().expect("_outlink not a string");
        let base = crate::wz::get_cached_base();
        let resolved = base.at_path(&path).expect("failed to resolve _outlink target");
        return extract_image(&resolved);
    }

    match wz_reader::property::png::get_image(&node.wz_node) {
        Ok(img) => img,
        Err(e) => panic!("extract_image failed at {}: {:?}", node.path(), e),
    }
}

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
