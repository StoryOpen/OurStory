pub mod asset_loader;
pub mod asset_source;

pub use wz::*;

use bevy::ecs::lifecycle::Add;
use bevy::ecs::observer::On;
use bevy::ecs::system::Commands;
use bevy::prelude::Vec2;
use bevy::sprite::{Anchor, Sprite};

/// Extension methods on `wz::Node` for reading Bevy types from WZ properties.
#[allow(dead_code)]
pub trait WzNodeExt {
    fn get_vec2_opt(&self, path: &str) -> Option<Vec2>;
    fn get_vec2_or(&self, path: &str, default: Vec2) -> Vec2;
}

impl WzNodeExt for crate::wz::Node {
    fn get_vec2_opt(&self, path: &str) -> Option<Vec2> {
        self.get_opt::<Vector2D>(path)
            .map(|v| Vec2::new(v.0 as f32, v.1 as f32))
    }

    #[allow(dead_code)]
    fn get_vec2_or(&self, path: &str, default: Vec2) -> Vec2 {
        self.get_vec2_opt(path).unwrap_or(default)
    }
}

/// Overrides the auto-inserted `Anchor::CENTER` (from `#[require(Anchor)]` on `Sprite`)
/// to `Anchor::BOTTOM_LEFT`. WZ origins are loaded as bottom-left-relative offsets in
/// Bevy Y-up space, so `BOTTOM_LEFT` is the correct anchor for the `pos - origin` formula.
pub fn set_sprite_bottom_left(trigger: On<Add, Sprite>, mut commands: Commands) {
    commands.entity(trigger.event().entity).insert(Anchor::BOTTOM_LEFT);
}
