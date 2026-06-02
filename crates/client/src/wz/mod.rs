pub mod asset_loader;
pub mod asset_source;

pub use wz::*;

use bevy::prelude::Vec2;

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
