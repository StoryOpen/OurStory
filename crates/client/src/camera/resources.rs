use bevy::prelude::*;

#[derive(Component)]
pub struct MainCamera;

#[derive(Resource, Clone, Copy)]
pub struct BaseResolution {
    pub width: f32,
    pub height: f32,
}

impl Default for BaseResolution {
    fn default() -> Self {
        Self { width: 1024.0, height: 768.0 }
    }
}
