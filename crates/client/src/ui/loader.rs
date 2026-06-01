use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use std::collections::HashMap;

use crate::wz::Node;

#[derive(Resource, Default)]
pub struct WzUiSpriteCache {
    handles: HashMap<String, Handle<Image>>,
}

impl WzUiSpriteCache {
    pub fn get_or_load(
        &mut self,
        node: &Node,
        wz_path: &str,
        images: &mut Assets<Image>,
    ) -> Handle<Image> {
        if let Some(handle) = self.handles.get(wz_path) {
            return handle.clone();
        }
        let dynamic_image: image::DynamicImage = node.clone().try_into().unwrap();
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
        let handle = images.add(image);
        self.handles.insert(wz_path.to_string(), handle.clone());
        handle
    }
}

pub struct UiSprite {
    pub handle: Handle<Image>,
    pub origin: Vec2,
}

pub fn load_ui_sprite(
    node: &Node,
    cache: &mut WzUiSpriteCache,
    images: &mut Assets<Image>,
) -> Option<UiSprite> {
    let origin: Vec2 = node.at_path("origin").ok()?.try_into().ok()?;
    let path = node.path();
    let handle = cache.get_or_load(node, &path, images);
    Some(UiSprite { handle, origin })
}

pub struct UiButtonSprites {
    pub normal: Handle<Image>,
    pub hover: Handle<Image>,
    pub pressed: Handle<Image>,
    pub disabled: Handle<Image>,
}

pub fn load_ui_button(
    button_node: &Node,
    cache: &mut WzUiSpriteCache,
    images: &mut Assets<Image>,
) -> Option<UiButtonSprites> {
    let normal = load_ui_sprite(&button_node.at_path("normal/0").ok()?, cache, images)?;
    let hover = load_ui_sprite(&button_node.at_path("mouseOver/0").ok()?, cache, images)?;
    let pressed = load_ui_sprite(&button_node.at_path("pressed/0").ok()?, cache, images)?;
    let disabled = load_ui_sprite(&button_node.at_path("disabled/0").ok()?, cache, images)?;
    Some(UiButtonSprites {
        normal: normal.handle,
        hover: hover.handle,
        pressed: pressed.handle,
        disabled: disabled.handle,
    })
}
