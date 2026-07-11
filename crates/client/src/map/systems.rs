use super::components::*;
use super::resources::*;
use bevy::prelude::*;

use crate::physics::{Foothold, FootholdGraph};
use crate::wz::map::WzMapAsset;

/// Handle for the default map asset, kicked off at startup.
#[derive(Resource)]
pub struct DefaultMapHandle(pub Handle<WzMapAsset>);

/// Startup: begin loading the default map via the derived `WzMapAsset`.
pub fn load_default_map(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load::<WzMapAsset>("wz://Map/Map/Map1/100010000.img.map");
    commands.insert_resource(DefaultMapHandle(handle));
}

/// Once the derived asset resolves, translate it into map state + physics.
/// Runs once.
pub fn poll_loaded_map(
    mut commands: Commands,
    default_map: Option<Res<DefaultMapHandle>>,
    assets: Res<Assets<WzMapAsset>>,
    mut current_map: ResMut<CurrentMap>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    let Some(handle) = default_map.as_ref().map(|h| &h.0) else { return };
    let Some(map) = assets.get(handle) else { return };

    let mut footholds: Vec<Foothold> = Vec::new();
    for layer in &map.foothold {
        for group in &layer.groups {
            for seg in &group.segments {
                let id = footholds.len() as i32;
                footholds.push(Foothold {
                    id,
                    next_id: if seg.next == 0 { None } else { Some(seg.next) },
                    prev_id: if seg.prev == 0 { None } else { Some(seg.prev) },
                    force: 0,
                    group: 0,
                    layer: 0,
                    x1: seg.x1 as f32,
                    y1: seg.y1 as f32,
                    x2: seg.x2 as f32,
                    y2: seg.y2 as f32,
                });
            }
        }
    }

    let bounds = MapBounds::from_footholds(&footholds);
    let graph = FootholdGraph::from_footholds(footholds);
    let foothold_count = graph.footholds.len();

    commands.insert_resource(bounds);
    commands.insert_resource(graph);

    *current_map = CurrentMap(MapState::Loaded {
        path: "Map/Map/Map1/100010000.img".to_string(),
        sprites: Vec::new(),
    });

    info!(
        "Map loaded (WzMapAsset): bgm={}, return_map={}, layers={}, back={}, life={}, portal={}, ladder={}, footholds={}, mini_map={}x{}",
        map.info.bgm,
        map.info.return_map,
        map.layers.len(),
        map.back.len(),
        map.life.len(),
        map.portal.len(),
        map.ladder_rope.len(),
        foothold_count,
        map.mini_map.width,
        map.mini_map.height,
    );

    *done = true;
}

pub fn tick_map_animations(
    time: Res<Time>,
    mut query: Query<(&mut MapAnimator, &mut Sprite, &mut Transform)>,
) {
    for (mut anim, mut sprite, mut transform) in &mut query {
        anim.timer.tick(time.delta());
        if !anim.timer.just_finished() {
            continue;
        }

        anim.current = (anim.current + 1) % anim.frames.len();
        let frame = &anim.frames[anim.current];

        sprite.image = frame.image.clone();
        sprite.flip_x = anim.flip;

        transform.translation = (anim.pos - frame.origin).extend(transform.translation.z);

        anim.timer = Timer::from_seconds(frame.delay.max(50) as f32 / 1000.0, TimerMode::Repeating);
    }
}

pub fn tick_move_effects(time: Res<Time>, mut query: Query<(&MapMoveEffect, &mut Transform)>) {
    let elapsed = time.elapsed_secs();

    for (effect, mut transform) in &mut query {
        let mut dx = 0.0f32;
        let mut dy = 0.0f32;
        let tau = std::f32::consts::TAU;

        match effect.move_type {
            1 => {
                dx = effect.move_w * (tau * 1000.0 * elapsed / effect.move_p).sin();
            }
            2 => {
                dy = effect.move_h * (tau * 1000.0 * elapsed / effect.move_p).sin();
            }
            3 => {
                let angle = tau * 1000.0 * elapsed / effect.move_r;
                transform.rotation = Quat::from_rotation_z(angle);
            }
            _ => {}
        }

        if effect.flow & 1 != 0 {
            dx += effect.rx as f32 * 5.0 * elapsed;
        }
        if effect.flow & 2 != 0 {
            dy += effect.ry as f32 * 5.0 * elapsed;
        }

        transform.translation = (effect.base + Vec2::new(dx, dy)).extend(transform.translation.z);
    }
}

pub fn tick_parallax_backgrounds(
    camera: Query<(&Camera, &GlobalTransform)>,
    mut backgrounds: Query<(&BackgroundMotion, &mut Transform), With<ParallaxBackground>>,
) {
    let Ok((_cam, cam_global)) = camera.single() else {
        return;
    };

    let cam_pos = cam_global.translation();
    for (bg, mut transform) in &mut backgrounds {
        let offset = parallax_offset(bg.rx, bg.ry, cam_pos);
        transform.translation = (bg.pos - bg.origin + offset).extend(transform.translation.z);
    }
}

pub fn tick_horizontal_tiled_parallax_backgrounds(
    camera: Query<(&Camera, &GlobalTransform)>,
    window: Query<&Window>,
    mut backgrounds: Query<
        (&BackgroundMotion, &BackgroundTile, &mut Transform),
        With<HorizontalTiledParallaxBackground>,
    >,
) {
    let Some((cam_pos, viewport)) = camera_view(&camera, &window) else {
        return;
    };

    for (bg, tile, mut transform) in &mut backgrounds {
        let offset = parallax_offset(bg.rx, bg.ry, cam_pos);
        position_tiled_background(
            &mut transform,
            tile,
            bg.pos - bg.origin,
            offset,
            cam_pos,
            viewport,
        );
    }
}

pub fn tick_vertical_tiled_parallax_backgrounds(
    camera: Query<(&Camera, &GlobalTransform)>,
    window: Query<&Window>,
    mut backgrounds: Query<
        (&BackgroundMotion, &BackgroundTile, &mut Transform),
        With<VerticalTiledParallaxBackground>,
    >,
) {
    let Some((cam_pos, viewport)) = camera_view(&camera, &window) else {
        return;
    };

    for (bg, tile, mut transform) in &mut backgrounds {
        let offset = parallax_offset(bg.rx, bg.ry, cam_pos);
        position_tiled_background(
            &mut transform,
            tile,
            bg.pos - bg.origin,
            offset,
            cam_pos,
            viewport,
        );
    }
}

pub fn tick_fully_tiled_parallax_backgrounds(
    camera: Query<(&Camera, &GlobalTransform)>,
    window: Query<&Window>,
    mut backgrounds: Query<
        (&BackgroundMotion, &BackgroundTile, &mut Transform),
        With<FullyTiledParallaxBackground>,
    >,
) {
    let Some((cam_pos, viewport)) = camera_view(&camera, &window) else {
        return;
    };

    for (bg, tile, mut transform) in &mut backgrounds {
        let offset = parallax_offset(bg.rx, bg.ry, cam_pos);
        position_tiled_background(
            &mut transform,
            tile,
            bg.pos - bg.origin,
            offset,
            cam_pos,
            viewport,
        );
    }
}

pub fn tick_horizontal_scrolling_backgrounds(
    camera: Query<(&Camera, &GlobalTransform)>,
    window: Query<&Window>,
    time: Res<Time>,
    mut backgrounds: Query<
        (&BackgroundMotion, &BackgroundTile, &mut Transform),
        With<HorizontalScrollingBackground>,
    >,
) {
    let Some((cam_pos, viewport)) = camera_view(&camera, &window) else {
        return;
    };

    let elapsed = time.elapsed_secs();
    for (bg, tile, mut transform) in &mut backgrounds {
        let offset = Vec2::new(
            bg.rx as f32 * 5.0 * elapsed,
            (bg.ry + 100) as f32 * cam_pos.y / 100.0,
        );
        position_tiled_background(
            &mut transform,
            tile,
            bg.pos - bg.origin,
            offset,
            cam_pos,
            viewport,
        );
    }
}

pub fn tick_vertical_scrolling_backgrounds(
    camera: Query<(&Camera, &GlobalTransform)>,
    window: Query<&Window>,
    time: Res<Time>,
    mut backgrounds: Query<
        (&BackgroundMotion, &BackgroundTile, &mut Transform),
        With<VerticalScrollingBackground>,
    >,
) {
    let Some((cam_pos, viewport)) = camera_view(&camera, &window) else {
        return;
    };

    let elapsed = time.elapsed_secs();
    for (bg, tile, mut transform) in &mut backgrounds {
        let offset = Vec2::new(
            (bg.rx + 100) as f32 * cam_pos.x / 100.0,
            bg.ry as f32 * 5.0 * elapsed,
        );
        position_tiled_background(
            &mut transform,
            tile,
            bg.pos - bg.origin,
            offset,
            cam_pos,
            viewport,
        );
    }
}

pub fn tick_fully_tiled_horizontal_scrolling_backgrounds(
    camera: Query<(&Camera, &GlobalTransform)>,
    window: Query<&Window>,
    time: Res<Time>,
    mut backgrounds: Query<
        (&BackgroundMotion, &BackgroundTile, &mut Transform),
        With<FullyTiledHorizontalScrollingBackground>,
    >,
) {
    let Some((cam_pos, viewport)) = camera_view(&camera, &window) else {
        return;
    };

    let elapsed = time.elapsed_secs();
    for (bg, tile, mut transform) in &mut backgrounds {
        let offset = Vec2::new(
            bg.rx as f32 * 5.0 * elapsed,
            (bg.ry + 100) as f32 * cam_pos.y / 100.0,
        );
        position_tiled_background(
            &mut transform,
            tile,
            bg.pos - bg.origin,
            offset,
            cam_pos,
            viewport,
        );
    }
}

pub fn tick_fully_tiled_vertical_scrolling_backgrounds(
    camera: Query<(&Camera, &GlobalTransform)>,
    window: Query<&Window>,
    time: Res<Time>,
    mut backgrounds: Query<
        (&BackgroundMotion, &BackgroundTile, &mut Transform),
        With<FullyTiledVerticalScrollingBackground>,
    >,
) {
    let Some((cam_pos, viewport)) = camera_view(&camera, &window) else {
        return;
    };

    let elapsed = time.elapsed_secs();
    for (bg, tile, mut transform) in &mut backgrounds {
        let offset = Vec2::new(
            (bg.rx + 100) as f32 * cam_pos.x / 100.0,
            bg.ry as f32 * 5.0 * elapsed,
        );
        position_tiled_background(
            &mut transform,
            tile,
            bg.pos - bg.origin,
            offset,
            cam_pos,
            viewport,
        );
    }
}

fn camera_view(
    camera: &Query<(&Camera, &GlobalTransform)>,
    window: &Query<&Window>,
) -> Option<(Vec3, Vec2)> {
    let Ok((_cam, cam_global)) = camera.single() else {
        return None;
    };
    let Ok(window) = window.single() else {
        return None;
    };

    Some((
        cam_global.translation(),
        Vec2::new(window.width(), window.height()),
    ))
}

fn parallax_offset(rx: i32, ry: i32, cam_pos: Vec3) -> Vec2 {
    Vec2::new(
        (rx + 100) as f32 * cam_pos.x / 100.0,
        (ry + 100) as f32 * cam_pos.y / 100.0,
    )
}

fn position_tiled_background(
    transform: &mut Transform,
    tile: &BackgroundTile,
    anchor: Vec2,
    offset: Vec2,
    cam_pos: Vec3,
    viewport: Vec2,
) {
    let viewport_left = cam_pos.x - viewport.x / 2.0;
    let viewport_bottom = cam_pos.y - viewport.y / 2.0;

    if tile.num_cols > 1 {
        let base_col = ((viewport_left - anchor.x - offset.x) / tile.spacing_x).floor();
        transform.translation.x =
            anchor.x + (base_col + tile.grid_col as f32) * tile.spacing_x + offset.x;
    } else {
        transform.translation.x = anchor.x + offset.x;
    }

    if tile.num_rows > 1 {
        let base_row = ((viewport_bottom - anchor.y - offset.y) / tile.spacing_y).floor();
        transform.translation.y =
            anchor.y + (base_row + tile.grid_row as f32) * tile.spacing_y + offset.y;
    } else {
        transform.translation.y = anchor.y + offset.y;
    }
}
