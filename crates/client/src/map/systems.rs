use bevy::{
    asset::AssetEvent,
    ecs::message::MessageReader,
    prelude::*,
};
use crate::physics::FootholdGraph;
use crate::wz::asset_loader::{BackgroundData, TileData, ObjData, WzMapAsset};
use super::components::*;
use super::events::*;
use super::resources::*;

pub fn handle_request_map(
    event: On<RequestMap>,
    mut current_map: ResMut<CurrentMap>,
    mut cache: ResMut<MapCache>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    let path = &event.event().0;

    if let CurrentMap(MapState::Loaded { path: cur, .. }) = &*current_map {
        if cur == path {
            return;
        }
    }

    if let Some(handle) = cache.get(path) {
        commands.trigger(MapReady { path: path.clone(), handle });
        return;
    }

    if let CurrentMap(MapState::Loading { path: cur, .. }) = &*current_map {
        if cur == path {
            return;
        }
    }

    let asset_path = format!("wz://{}.map", path);
    let handle = asset_server.load::<WzMapAsset>(&asset_path);
    *current_map = CurrentMap(MapState::Loading { path: path.clone(), handle });
}

pub fn on_asset_loaded(
    mut ev_asset: MessageReader<AssetEvent<WzMapAsset>>,
    current_map: Res<CurrentMap>,
    mut cache: ResMut<MapCache>,
    mut commands: Commands,
) {
    let (loading_path, loading_handle) = match &*current_map {
        CurrentMap(MapState::Loading { path, handle }) => (path.clone(), handle.clone()),
        _ => return,
    };

    for ev in ev_asset.read() {
        if let AssetEvent::LoadedWithDependencies { id } = ev {
            if *id == loading_handle.id() {
                cache.insert(loading_path.clone(), loading_handle.clone());
                commands.trigger(MapReady { path: loading_path, handle: loading_handle });
                break;
            }
        }
    }
}

pub fn spawn_map(
    trigger: On<MapReady>,
    mut commands: Commands,
    assets: Res<Assets<WzMapAsset>>,
    images: Res<Assets<Image>>,
    window: Query<&Window>,
    mut current_map: ResMut<CurrentMap>,
) {
    let ev = trigger.event();
    let asset = assets.get(&ev.handle).expect("WzMapAsset must be loaded");

    let old = std::mem::replace(&mut *current_map, CurrentMap(MapState::None));
    if let CurrentMap(MapState::Loaded { sprites, .. }) = old {
        for e in sprites {
            commands.entity(e).despawn();
        }
    }

    let bounds = compute_bounds(&asset.info, &asset.footholds);
    commands.insert_resource(bounds);
    let graph = FootholdGraph::from_footholds(asset.footholds.clone());
    commands.insert_resource(graph);
    commands.trigger(super::events::MapLoaded { path: ev.path.clone(), bounds });

    let viewport = window.single().map(|w| Vec2::new(w.width(), w.height())).unwrap_or(Vec2::new(1920.0, 1080.0));

    let mut z = -1000.0;
    let mut sprites = Vec::new();

    for b in &asset.backgrounds {
        z += 1.0;
        let tex_size = images.get(&b.image).map(|i| i.size_f32()).unwrap_or(Vec2::splat(128.0));
        let mut ents = spawn_background_entity(b, &mut commands, z, viewport, tex_size);
        sprites.append(&mut ents);
    }

    info!("spawned {} sprites for map {}", sprites.len(), ev.path);
    *current_map = CurrentMap(MapState::Loaded { path: ev.path.clone(), sprites, handle: ev.handle.clone() });
}

fn compute_bounds(info: &crate::wz::asset_loader::MapInfo, footholds: &[crate::wz::asset_loader::Foothold]) -> super::resources::MapBounds {
    if let (Some(l), Some(r), Some(t), Some(b)) = (info.vr_left, info.vr_right, info.vr_top, info.vr_bottom) {
        super::resources::MapBounds::from_vr(l, r, t, b)
    } else if !footholds.is_empty() {
        super::resources::MapBounds::from_footholds(footholds)
    } else {
        super::resources::MapBounds { left: -1000.0, right: 1000.0, top: 1000.0, bottom: -1000.0 }
    }
}

#[allow(dead_code)]
fn spawn_tile_entity(tile: &TileData, commands: &mut Commands, z: f32) -> Entity {
    let mut entity = commands.spawn((
        Sprite::from_image(tile.image.clone()),
        Transform::from_translation((tile.pos - tile.origin).extend(z)),
        MapSprite,
    ));

    if !tile.animation_frames.is_empty() {
        let delay = tile.animation_frames[0].delay.max(50);
        entity.insert(MapAnimator {
            frames: tile.animation_frames.clone(),
            current: 0,
            timer: Timer::from_seconds(delay as f32 / 1000.0, TimerMode::Repeating),
            base: tile.pos,
            flip: false,
        });
    }

    entity.id()
}

#[allow(dead_code)]
fn spawn_obj_entity(obj: &ObjData, commands: &mut Commands, z: f32) -> Entity {
    let mut entity = commands.spawn((
        Sprite {
            image: obj.image.clone(),
            flip_x: obj.flip,
            ..default()
        },
        Transform::from_translation((obj.pos - obj.origin).extend(z)),
        MapSprite,
    ));

    if !obj.animation_frames.is_empty() {
        let delay = obj.animation_frames[0].delay.max(50);
        entity.insert(MapAnimator {
            frames: obj.animation_frames.clone(),
            current: 0,
            timer: Timer::from_seconds(delay as f32 / 1000.0, TimerMode::Repeating),
            base: obj.pos,
            flip: obj.flip,
        });
    }

    if obj.flow != 0 || obj.animation_frames.iter().any(|f| f.move_type != 0) {
        let move_type = obj.animation_frames.first()
            .map(|f| f.move_type)
            .unwrap_or(0);
        let move_w = obj.animation_frames.first()
            .map(|f| f.move_w)
            .unwrap_or(0.0);
        let move_h = obj.animation_frames.first()
            .map(|f| f.move_h)
            .unwrap_or(0.0);
        let move_p = obj.animation_frames.first()
            .map(|f| f.move_p)
            .unwrap_or(6283.0);
        let move_r = obj.animation_frames.first()
            .map(|f| f.move_r)
            .unwrap_or(0.0);
        let a0 = obj.animation_frames.first()
            .map(|f| f.a0)
            .unwrap_or(1.0);
        let a1 = obj.animation_frames.first()
            .map(|f| f.a1)
            .unwrap_or(1.0);

        entity.insert(MapMoveEffect {
            base: obj.pos,
            move_type, move_w, move_h, move_p, move_r,
            a0, a1,
            flow: obj.flow,
            rx: obj.rx, ry: obj.ry,
            cx: obj.cx, cy: obj.cy,
        });
    }

    entity.id()
}

fn spawn_background_entity(
    b: &BackgroundData,
    commands: &mut Commands,
    z: f32,
    viewport: Vec2,
    tex_size: Vec2,
) -> Vec<Entity> {
    let tile_x = matches!(b.btype, 1 | 3 | 4 | 6 | 7);
    let tile_y = matches!(b.btype, 2 | 3 | 5 | 6 | 7);

    let bg = MapParallaxBackground {
        pos: b.pos,
        origin: b.origin,
        rx: b.rx,
        ry: b.ry,
        btype: b.btype,
        cx: b.cx,
        cy: b.cy,
        alpha: b.alpha,
        flip: b.flip,
        front: b.front,
    };

    if !tile_x && !tile_y {
        let mut entity = commands.spawn((
            Sprite {
                image: b.image.clone(),
                flip_x: b.flip,
                ..default()
            },
            Transform::from_translation((b.pos - b.origin).extend(z)),
            bg,
            MapSprite,
        ));

        if !b.animation_frames.is_empty() {
            let delay = b.animation_frames[0].delay.max(50);
            entity.insert(MapAnimator {
                frames: b.animation_frames.clone(),
                current: 0,
                timer: Timer::from_seconds(delay as f32 / 1000.0, TimerMode::Repeating),
                base: b.pos,
                flip: b.flip,
            });
        }

        return vec![entity.id()];
    }

    let spacing_x: f32 = if b.cx == 0 { tex_size.x } else { b.cx.unsigned_abs() as f32 };
    let spacing_y: f32 = if b.cy == 0 { tex_size.y } else { b.cy.unsigned_abs() as f32 };

    let num_cols = if tile_x { (viewport.x / spacing_x).ceil() as i32 + 2 } else { 1 };
    let num_rows = if tile_y { (viewport.y / spacing_y).ceil() as i32 + 2 } else { 1 };

    let grid_w = num_cols as f32 * spacing_x;
    let grid_h = num_rows as f32 * spacing_y;
    let center = b.pos - b.origin;

    let mut entities = Vec::with_capacity((num_cols * num_rows) as usize);
    for row in 0..num_rows {
        for col in 0..num_cols {
            let tx = col as f32 * spacing_x - grid_w / 2.0;
            let ty = row as f32 * spacing_y - grid_h / 2.0;

            let mut entity = commands.spawn((
                Sprite {
                    image: b.image.clone(),
                    flip_x: b.flip,
                    ..default()
                },
                Transform::from_translation(Vec3::new(
                    center.x + tx,
                    center.y + ty,
                    z,
                )),
                bg.clone(),
                BackgroundTile {
                    grid_col: col,
                    grid_row: row,
                    num_cols,
                    num_rows,
                    spacing_x,
                    spacing_y,
                },
                MapSprite,
            ));

            if !b.animation_frames.is_empty() {
                let delay = b.animation_frames[0].delay.max(50);
                entity.insert(MapAnimator {
                    frames: b.animation_frames.clone(),
                    current: 0,
                    timer: Timer::from_seconds(delay as f32 / 1000.0, TimerMode::Repeating),
                    base: b.pos,
                    flip: b.flip,
                });
            }

            entities.push(entity.id());
        }
    }

    entities
}

pub fn tick_map_animations(
    time: Res<Time>,
    mut query: Query<(&mut MapAnimator, &mut Sprite, &mut Transform, Option<&BackgroundTile>)>,
) {
    for (mut anim, mut sprite, mut transform, tile_opt) in &mut query {
        anim.timer.tick(time.delta());
        if !anim.timer.just_finished() {
            continue;
        }

        anim.current = (anim.current + 1) % anim.frames.len();
        let frame = &anim.frames[anim.current];

        sprite.image = frame.image.clone();
        sprite.flip_x = anim.flip;

        // Background tile positions are managed by tick_background_parallax — don't fight it
        if tile_opt.is_none() {
            transform.translation = (anim.base - frame.origin).extend(transform.translation.z);
        }

        anim.timer = Timer::from_seconds(
            frame.delay.max(50) as f32 / 1000.0,
            TimerMode::Repeating,
        );
    }
}

pub fn tick_move_effects(
    time: Res<Time>,
    mut query: Query<(&MapMoveEffect, &mut Transform)>,
) {
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

pub fn tick_background_parallax(
    camera: Query<(&Camera, &GlobalTransform)>,
    window: Query<&Window>,
    time: Res<Time>,
    mut backgrounds: Query<(&MapParallaxBackground, &mut Transform, &mut Sprite, Option<&BackgroundTile>)>,
) {
    let Ok((_cam, cam_global)) = camera.single() else {
        return;
    };
    let Ok(window) = window.single() else {
        return;
    };

    let cam_pos = cam_global.translation();
    let viewport = Vec2::new(window.width(), window.height());
    let elapsed = time.elapsed_secs();

    for (bg, mut transform, mut sprite, tile_opt) in &mut backgrounds {
        let rx = bg.rx as f32;
        let ry = bg.ry as f32;

        sprite.flip_x = bg.flip;

        let Some(tile) = tile_opt else {
            // Non-tiled: compute exact world position from parallax (original behavior)
            let shift_x = rx * cam_pos.x / 100.0 + viewport.x / 2.0;
            let shift_y = -ry * cam_pos.y / 100.0 + viewport.y / 2.0;

            let (pivot_screen_x, pivot_screen_y) = match bg.btype {
                0 | 1 | 2 | 3 => (bg.pos.x + shift_x, -bg.pos.y + shift_y),
                4 | 6 => (
                    bg.pos.x + rx * 5.0 * elapsed - cam_pos.x,
                    -bg.pos.y + shift_y,
                ),
                5 | 7 => (
                    bg.pos.x + shift_x,
                    -bg.pos.y + ry * 5.0 * elapsed + cam_pos.y,
                ),
                _ => (bg.pos.x, -bg.pos.y),
            };

            let bevy_x = pivot_screen_x + cam_pos.x - viewport.x / 2.0 - bg.origin.x;
            let bevy_y = cam_pos.y + viewport.y / 2.0 - pivot_screen_y + bg.origin.y;

            transform.translation.x = bevy_x;
            transform.translation.y = bevy_y;
            continue;
        };

        // Tiled background: grid is viewport-aligned so tiles always cover screen.
        // Intra-grid scroll offset creates the parallax / auto-scroll effect.
        let offset_x = match bg.btype {
            0 | 1 | 2 | 3 | 5 | 7 => rx * cam_pos.x / 100.0,
            4 | 6 => rx * 5.0 * elapsed,
            _ => 0.0,
        };
        let offset_y = match bg.btype {
            0 | 1 | 2 | 3 | 4 | 6 => -ry * cam_pos.y / 100.0,
            5 | 7 => ry * 5.0 * elapsed,
            _ => 0.0,
        };

        let grid_w = tile.num_cols as f32 * tile.spacing_x;
        let grid_h = tile.num_rows as f32 * tile.spacing_y;

        // Use viewport center so grid stays visible regardless of camera position.
        // Tile position wraps within the grid: when one scrolls past the edge it
        // reappears on the opposite side, giving the illusion of infinite tiles.
        let tx = (tile.grid_col as f32 * tile.spacing_x + offset_x).rem_euclid(grid_w.max(1.0));
        let ty = (tile.grid_row as f32 * tile.spacing_y + offset_y).rem_euclid(grid_h.max(1.0));

        transform.translation.x = cam_pos.x - grid_w / 2.0 + tx;
        transform.translation.y = cam_pos.y - grid_h / 2.0 + ty;
    }
}

pub fn draw_background_gizmos(
    mut gizmos: Gizmos,
    images: Res<Assets<Image>>,
    backgrounds: Query<(&MapParallaxBackground, &Transform, &Sprite, Option<&BackgroundTile>)>,
) {
    const CROSS_HALF: f32 = 20.0;

    for (bg, transform, sprite, tile_opt) in &backgrounds {
        let color = bg_gizmo_color(bg.btype, bg.front);
        let pos = transform.translation.truncate();

        gizmos.line_2d(pos - Vec2::new(CROSS_HALF, 0.0), pos + Vec2::new(CROSS_HALF, 0.0), color);
        gizmos.line_2d(pos - Vec2::new(0.0, CROSS_HALF), pos + Vec2::new(0.0, CROSS_HALF), color);

        let (w, h) = if let Some(tile) = tile_opt {
            (tile.spacing_x, tile.spacing_y)
        } else {
            let tex_size = images.get(&sprite.image).map(|i| i.size_f32()).unwrap_or(Vec2::splat(64.0));
            (tex_size.x, tex_size.y)
        };

        gizmos.rect_2d(pos + Vec2::new(w, h) / 2.0, Vec2::new(w, h), color);
    }
}

fn bg_gizmo_color(btype: i32, front: bool) -> Color {
    let (r, g, b) = match btype {
        0 => (0.2, 1.0, 0.2),
        1 => (1.0, 1.0, 0.2),
        2 => (0.2, 0.6, 1.0),
        3 => (0.2, 1.0, 1.0),
        4 => (1.0, 0.6, 0.2),
        5 => (1.0, 0.2, 0.2),
        6 => (1.0, 0.2, 1.0),
        7 => (1.0, 0.6, 0.8),
        _ => (0.8, 0.8, 0.8),
    };
    let alpha = if front { 0.95 } else { 0.5 };
    Color::srgba(r, g, b, alpha)
}
