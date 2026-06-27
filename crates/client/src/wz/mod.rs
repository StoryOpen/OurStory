pub mod asset_loaders;
pub mod asset_source;

use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages,
    ecs::lifecycle::Add,
    ecs::observer::On,
    ecs::system::Commands,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    sprite::{Anchor, Sprite},
};

pub fn set_sprite_bottom_left(trigger: On<Add, Sprite>, mut commands: Commands) {
    commands
        .entity(trigger.event().entity)
        .insert(Anchor::BOTTOM_LEFT);
}

#[derive(Resource, Default)]
pub struct WzImageCache {
    cache: HashMap<String, Handle<Image>>,
}

impl WzImageCache {
    pub fn get(&self, path: &str) -> Option<Handle<Image>> {
        self.cache.get(path).cloned()
    }

    pub fn get_or_load(
        &mut self,
        path: &str,
        images: &mut Assets<Image>,
    ) -> Handle<Image> {
        if let Some(handle) = self.cache.get(path) {
            return handle.clone();
        }
        let wz = wz::WzData::global();
        let dynamic_image = wz.load_image(path).unwrap_or_else(|e| {
            panic!("failed to load image at {path}: {e}")
        });
        let rgba = dynamic_image.to_rgba8();
        let (width, height) = rgba.dimensions();
        let image = Image::new(
            Extent3d { width, height, depth_or_array_layers: 1 },
            TextureDimension::D2,
            rgba.into_raw(),
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        let handle = images.add(image);
        self.cache.insert(path.to_string(), handle.clone());
        handle
    }
}
