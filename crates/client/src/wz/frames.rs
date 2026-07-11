use bevy::prelude::*;
use bevy_asset::RenderAssetUsages;
use wz_derive::WzAsset;

/// A single frame in a WZ sprite animation.
/// The node itself is a PNG image; `origin`, `z`, and `delay` are its children.
#[derive(Clone, Debug, WzAsset)]
pub struct WzFrameAsset {
    #[wz(origin)]
    pub origin: Vec2,
    #[wz(default = 0)]
    pub z: i32,
    #[wz(default = 100)]
    pub delay: i32,
    #[wz(image)]
    pub image: Handle<Image>,
}

/// A sequence of frames forming a WZ sprite animation.
/// Each numeric child of the WZ node is a frame (PNG with origin/z/delay).
#[derive(Asset, TypePath, Clone, Debug, WzAsset)]
#[wz(ext = "frame-anim", path = ".")]
pub struct WzFrameAnimationAsset {
    #[wz(children(numeric_only))]
    pub frames: Vec<WzFrameAsset>,
}
