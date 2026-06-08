pub mod asset_source;
pub mod foothold;

pub use wz::*;

use bevy::ecs::lifecycle::Add;
use bevy::ecs::observer::On;
use bevy::ecs::system::Commands;
use bevy::prelude::Vec2;
use bevy::sprite::{Anchor, Sprite};

/// Extension trait for converting `Vector2D` to Bevy `Vec2`.
pub trait Vector2DExt {
    fn to_vec2(self) -> Vec2;
}

impl Vector2DExt for Vector2D {
    fn to_vec2(self) -> Vec2 {
        Vec2::new(self.0 as f32, self.1 as f32)
    }
}

/// Extension methods on `wz::Node` for reading Bevy types from WZ properties.
#[allow(dead_code)]
pub trait WzNodeExt {
    fn try_get_pos(&self) -> Result<Vec2, NodeError>;
}

impl WzNodeExt for crate::wz::Node {
    fn try_get_pos(&self) -> Result<Vec2, NodeError> {
        let v = self.read_pos()?;
        Ok(Vec2::new(v.0 as f32, v.1 as f32))
    }
}

/// Overrides the auto-inserted `Anchor::CENTER` (from `#[require(Anchor)]` on `Sprite`)
/// to `Anchor::BOTTOM_LEFT`. WZ origins are loaded as bottom-left-relative offsets in
/// Bevy Y-up space, so `BOTTOM_LEFT` is the correct anchor for the `pos - origin` formula.
pub fn set_sprite_bottom_left(trigger: On<Add, Sprite>, mut commands: Commands) {
    commands
        .entity(trigger.event().entity)
        .insert(Anchor::BOTTOM_LEFT);
}
