use bevy::prelude::*;

#[derive(Clone)]
pub struct MapAnimFrame {
    pub image: Handle<Image>,
    pub origin: Vec2,
    pub delay: u32,
}

#[derive(Component)]
pub struct MapAnimator {
    pub frames: Vec<MapAnimFrame>,
    pub current: usize,
    pub timer: Timer,
    pub pos: Vec2,
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

#[derive(Clone, Component)]
pub struct BackgroundMotion {
    pub pos: Vec2,
    pub origin: Vec2,
    pub rx: i32,
    pub ry: i32,
}

#[derive(Component)]
pub struct ParallaxBackground;

#[derive(Component)]
pub struct HorizontalTiledParallaxBackground;

#[derive(Component)]
pub struct VerticalTiledParallaxBackground;

#[derive(Component)]
pub struct FullyTiledParallaxBackground;

#[derive(Component)]
pub struct HorizontalScrollingBackground;

#[derive(Component)]
pub struct VerticalScrollingBackground;

#[derive(Component)]
pub struct FullyTiledHorizontalScrollingBackground;

#[derive(Component)]
pub struct FullyTiledVerticalScrollingBackground;

#[derive(Component)]
pub struct BackgroundTile {
    pub grid_col: i32,
    pub grid_row: i32,
    pub num_cols: i32,
    pub num_rows: i32,
    pub spacing_x: f32,
    pub spacing_y: f32,
}

#[derive(Component)]
pub struct Portal;
