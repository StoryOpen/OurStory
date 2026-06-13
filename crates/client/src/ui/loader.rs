use bevy::prelude::*;

pub use crate::wz::WzImageCache;

pub struct UiSprite {
    pub handle: Handle<Image>,
    pub origin: Vec2,
}

pub fn load_ui_sprite(
    path: &str,
    cache: &mut WzImageCache,
    images: &mut Assets<Image>,
) -> Option<UiSprite> {
    let wz = wz::WzData::global();
    let origin_v = wz.load_origin(path).ok()?;
    let origin = Vec2::new(origin_v.0, origin_v.1);
    let handle = cache.get_or_load(path, images);
    Some(UiSprite { handle, origin })
}

pub struct UiButtonSprites {
    pub normal: Handle<Image>,
    pub hover: Handle<Image>,
    pub pressed: Handle<Image>,
    pub disabled: Handle<Image>,
}

pub fn load_ui_button(
    button_path: &str,
    cache: &mut WzImageCache,
    images: &mut Assets<Image>,
) -> Option<UiButtonSprites> {
    let normal = load_ui_sprite(&format!("{button_path}/normal/0"), cache, images)?;
    let hover = load_ui_sprite(&format!("{button_path}/mouseOver/0"), cache, images)?;
    let pressed = load_ui_sprite(&format!("{button_path}/pressed/0"), cache, images)?;
    let disabled = load_ui_sprite(&format!("{button_path}/disabled/0"), cache, images)?;
    Some(UiButtonSprites {
        normal: normal.handle,
        hover: hover.handle,
        pressed: pressed.handle,
        disabled: disabled.handle,
    })
}
