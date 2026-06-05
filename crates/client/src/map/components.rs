use bevy::prelude::*;

use crate::wz::asset_loader::AnimFrame;

#[derive(Component)]
pub struct MapAnimator {
    pub frames: Vec<AnimFrame>,
    pub current: usize,
    pub timer: Timer,
    pub base: Vec2,
    pub flip: bool,
}

#[allow(dead_code)]
#[derive(Component)]
pub struct MapMoveEffect {
    pub base: Vec2,
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
#[derive(Clone, Component)]
pub struct MapParallaxBackground {
    pub pos: Vec2,
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

#[derive(Component)]
pub struct BackgroundTile {
    pub grid_col: i32,
    pub grid_row: i32,
    pub num_cols: i32,
    pub num_rows: i32,
    pub spacing_x: f32,
    pub spacing_y: f32,
}

/// Marker component for map sprites that should be despawned on map change.
#[derive(Component)]
pub struct MapSprite;
