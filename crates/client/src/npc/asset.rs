use std::collections::{BTreeMap, HashMap};

use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::*,
    reflect::TypePath,
};
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
                let image_handle = crate::wz::load_or_decode_image(&frame_node, load_context, label);
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


