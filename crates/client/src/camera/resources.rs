use bevy::prelude::*;

#[derive(Component)]
pub struct MainCamera;

#[derive(Resource, Clone, Copy)]
pub struct BaseResolution {
    pub width: f32,
    pub height: f32,
}
