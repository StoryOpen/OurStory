use bevy::prelude::*;

use crate::wz::asset_loader::AnimFrame;

#[derive(Component)]
pub struct MapAnimator {
    pub frames: Vec<AnimFrame>,
    pub current: usize,
    pub timer: Timer,
    pub base_x: f32,
    pub base_y: f32,
    pub flip: bool,
}

#[allow(dead_code)]
#[derive(Component)]
pub struct MapMoveEffect {
    pub base_x: f32,
    pub base_y: f32,
    pub move_type: i32,
    pub move_w: f32,
    pub move_h: f32,
    pub move_p: f32,
    pub move_r: f32,
    pub a0: f32,
    pub a1: f32,
    pub flow: i32,
    pub rx: i32,
    pub ry: i32,
    pub cx: i32,
    pub cy: i32,
}

#[allow(dead_code)]
#[derive(Component)]
pub struct MapParallaxBackground {
    pub base_x: f32,
    pub base_y: f32,
    pub origin: Vec2,
    pub rx: i32,
    pub ry: i32,
    pub btype: i32,
    pub cx: i32,
    pub cy: i32,
    pub alpha: u8,
    pub flip: bool,
    pub front: bool,
}

/// Marker component for map sprites that should be despawned on map change.
#[derive(Component)]
pub struct MapSprite;
