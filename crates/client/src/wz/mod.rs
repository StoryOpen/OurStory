pub mod asset_source;
pub mod foothold;

pub use wz::*;

use bevy::{
    asset::{Handle, LoadContext, RenderAssetUsages},
    ecs::lifecycle::Add,
    ecs::observer::On,
    ecs::system::Commands,
    image::Image,
    math::Vec2,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    sprite::{Anchor, Sprite},
};

/// Extension trait for converting `Vector2D` to Bevy `Vec2`.
pub trait Vector2DExt {
    fn to_vec2(self) -> Vec2;
}

impl Vector2DExt for Vector2D {
    fn to_vec2(self) -> Vec2 {
        Vec2::new(self.0 as f32, self.1 as f32)
    }
}

/// Load a PNG image from a WZ node into a Bevy asset, with dedup by label.
/// Transparently follows `_inlink`/`_outlink` references.
pub fn load_or_decode_image(
    node: &Node,
    load_context: &mut LoadContext<'_>,
    label: String,
) -> Handle<Image> {
    if load_context.has_labeled_asset(&label) {
        return load_context.get_label_handle::<Image>(&label);
    }
    let dynamic_image = node.extract_image().unwrap_or_else(|e| {
        panic!("failed to extract image at {}: {e}", node.path())
    });
    let rgba = dynamic_image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba.into_raw(),
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    load_context.add_labeled_asset(label, image)
}

/// Overrides the auto-inserted `Anchor::CENTER` (from `#[require(Anchor)]` on `Sprite`)
/// to `Anchor::BOTTOM_LEFT`. WZ origins are loaded as bottom-left-relative offsets in
/// Bevy Y-up space, so `BOTTOM_LEFT` is the correct anchor for the `pos - origin` formula.
pub fn set_sprite_bottom_left(trigger: On<Add, Sprite>, mut commands: Commands) {
    commands
        .entity(trigger.event().entity)
        .insert(Anchor::BOTTOM_LEFT);
}
