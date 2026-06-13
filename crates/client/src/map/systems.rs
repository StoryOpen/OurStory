use crate::wz::asset_loaders::WzMapAsset;
use super::components::*;
use super::events::*;
use super::resources::*;
use crate::layer::GameLayer;
#[cfg(feature = "mob")]
use crate::mob::events::SpawnMob;
#[cfg(feature = "npc")]
use crate::npc::events::SpawnNpc;
use crate::physics::FootholdGraph;
use bevy::{
    asset::{AssetEvent, AssetLoadFailedEvent},
    asset::RenderAssetUsages,
    ecs::message::MessageReader,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use std::collections::HashMap;
use std::backtrace::Backtrace;
use std::fs::OpenOptions;
use std::io::Write;

fn despawn_current_sprites(current_map: &mut CurrentMap, commands: &mut Commands) {
    let old = std::mem::replace(current_map, CurrentMap(MapState::None));
    if let CurrentMap(MapState::Loaded { sprites, .. }) = old {
        for e in sprites {
            commands.entity(e).despawn();
        }
    }
}

fn log_map_failure(path: &str, context: &str, message: &str) {
    error!("failed to load map {}: {}: {}", path, context, message);

    let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("map_panics.log")
    else {
        return;
    };

    let _ = writeln!(file, "map: {path}");
    let _ = writeln!(file, "context: {context}");
    let _ = writeln!(file, "message: {message}");
    let _ = writeln!(file, "backtrace:\n{}", Backtrace::force_capture());
    let _ = writeln!(file, "---");
}

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
        despawn_current_sprites(&mut current_map, &mut commands);
        *current_map = CurrentMap(MapState::Loading {
            path: path.clone(),
            handle: handle.clone(),
        });
        commands.trigger(MapReady {
            path: path.clone(),
            handle,
        });
        return;
    }

    if let CurrentMap(MapState::Loading { path: cur, .. }) = &*current_map {
        if cur == path {
            return;
        }
    }
    if let CurrentMap(MapState::Clearing { path: cur, .. }) = &*current_map {
        if cur == path {
            return;
        }
    }

    let asset_path = format!("wz://{}.map", path);
    let handle = asset_server.load::<WzMapAsset>(&asset_path);
    despawn_current_sprites(&mut current_map, &mut commands);
    *current_map = CurrentMap(MapState::Loading {
        path: path.clone(),
        handle,
    });
}

pub fn on_asset_loaded(
    mut ev_asset: MessageReader<AssetEvent<WzMapAsset>>,
    mut ev_failed: MessageReader<AssetLoadFailedEvent<WzMapAsset>>,
    mut current_map: ResMut<CurrentMap>,
    mut cache: ResMut<MapCache>,
    mut commands: Commands,
) {
    let (loading_path, loading_handle, ready) = match &*current_map {
        CurrentMap(MapState::Loading { path, handle }) => (path.clone(), handle.clone(), true),
        CurrentMap(MapState::Clearing {
            path,
            handle,
            ready,
        }) => (path.clone(), handle.clone(), *ready),
        _ => return,
    };

    for ev in ev_failed.read() {
        if ev.id == loading_handle.id() {
            log_map_failure(&loading_path, "asset load", &ev.error.to_string());
            *current_map = CurrentMap(MapState::Failed { path: loading_path });
            return;
        }
    }

    for ev in ev_asset.read() {
        if let AssetEvent::LoadedWithDependencies { id } = ev {
            if *id == loading_handle.id() {
                cache.insert(loading_path.clone(), loading_handle.clone());
                if ready {
                    commands.trigger(MapReady {
                        path: loading_path,
                        handle: loading_handle,
                    });
                } else {
                    *current_map = CurrentMap(MapState::Clearing {
                        path: loading_path,
                        handle: loading_handle,
                        ready: true,
                    });
                }
                break;
            }
        }
    }
}

pub fn spawn_map(
    trigger: On<MapReady>,
    mut commands: Commands,
    assets: Res<Assets<WzMapAsset>>,
    mut images: ResMut<Assets<Image>>,
    window: Query<&Window>,
    mut current_map: ResMut<CurrentMap>,
) {
    let ev = trigger.event();
    let asset = assets.get(&ev.handle).expect("WzMapAsset must be loaded");
    let map = &asset.0;

    let old = std::mem::replace(&mut *current_map, CurrentMap(MapState::None));
    if let CurrentMap(MapState::Loaded { sprites, .. }) = old {
        for e in sprites {
            commands.entity(e).despawn();
        }
    }

    let bounds = compute_bounds(&map.info, &map.footholds);
    commands.insert_resource(bounds);
    let graph = FootholdGraph::from_footholds(map.footholds.clone());
    commands.insert_resource(graph);
    commands.trigger(super::events::MapLoaded {
        path: ev.path.clone(),
        bounds,
        handle: ev.handle.clone(),
    });

    let viewport = window
        .single()
        .map(|w| Vec2::new(w.width(), w.height()))
        .unwrap();
    info!("{:?}", viewport);

    let mut sprites = Vec::new();
    let mut image_cache = HashMap::new();

    for b in map.backgrounds.iter() {
        let handle = load_map_image(&b.image_path, &mut image_cache, &mut images);
        let tex_size = images.get(&handle).map(|i| i.size_f32()).unwrap_or_default();
        let z = if b.front {
            GameLayer::Foreground.with_offset(b.index as f32)
        } else {
            GameLayer::Background.with_offset(b.index as f32)
        };
        let mut ents = spawn_background_entity(b, &mut commands, z, viewport, tex_size, &mut image_cache, &mut images);
        sprites.append(&mut ents);
    }

    let max_span = (0..8usize)
        .map(|i| {
            let layer = map.layers.get(i);
            let (objs, tiles) = match layer {
                Some(l) => (&l.objs[..], &l.tiles[..]),
                None => return 0,
            };
            if objs.is_empty() && tiles.is_empty() {
                return 0;
            }
            let obj_max = objs.iter().map(|o| o.z).max().unwrap_or(0);
            let tile_max = tiles.iter().map(|t| t.z).max().unwrap_or(0);
            ((obj_max + tile_max + 2) as f32).max(1.0) as i32
        })
        .max()
        .unwrap_or(0) as f32;
    if max_span == 0.0 {
        return;
    }
    info!("max_span = {}", max_span);

    for layer_idx in 0..8usize {
        let Some(layer) = map.layers.get(layer_idx) else { continue };
        if layer.objs.is_empty() && layer.tiles.is_empty() {
            continue;
        }
        let layer_z = layer_idx as f32 * max_span;
        let obj_max_z = layer.objs.iter().map(|o| o.z).max().unwrap_or(0) as f32;
        let tile_base = if layer.objs.is_empty() { 0.0 } else { obj_max_z + 1.0 };

        for obj in &layer.objs {
            sprites.push(spawn_obj_entity(obj, &mut commands, layer_z + obj.z as f32, &mut image_cache, &mut images));
        }
        for tile in &layer.tiles {
            sprites.push(spawn_tile_entity(tile, &mut commands, layer_z + tile_base + tile.z as f32 + 0.5, &mut image_cache, &mut images));
        }
    }

    spawn_life(&map.life, &mut commands);

    info!("spawned {} sprites for map {}", sprites.len(), ev.path);
    *current_map = CurrentMap(MapState::Loaded {
        path: ev.path.clone(),
        sprites,
        handle: ev.handle.clone(),
    });
}

fn spawn_life(life: &[wz::LifeSpawn], commands: &mut Commands) {
    for entry in life {
        let pos = Vec2::new(entry.pos.0, entry.pos.1);
        match entry.spawn_type.as_str() {
            #[cfg(feature = "mob")]
            "m" => {
                commands.trigger(SpawnMob {
                    mob_id: entry.id,
                    x: pos.x,
                    y: pos.y,
                    z: 0,
                });
            }
            #[cfg(feature = "npc")]
            "n" => {
                commands.trigger(SpawnNpc {
                    npc_id: entry.id,
                    x: pos.x,
                    y: pos.y,
                    z: 0,
                    flip: entry.flip,
                });
            }
            _ => {}
        }
    }
}

fn compute_bounds(
    info: &wz::MapInfo,
    footholds: &[wz::Foothold],
) -> MapBounds {
    if let (Some(l), Some(r), Some(t), Some(b)) =
        (info.vr_left, info.vr_right, info.vr_top, info.vr_bottom)
    {
        MapBounds::from_vr(l, r, t, b)
    } else if !footholds.is_empty() {
        MapBounds::from_footholds(footholds)
    } else {
        MapBounds {
            left: -1000.0,
            right: 1000.0,
            top: 1000.0,
            bottom: -1000.0,
        }
    }
}

fn load_map_image(
    path: &str,
    cache: &mut HashMap<String, Handle<Image>>,
    images: &mut Assets<Image>,
) -> Handle<Image> {
    if let Some(handle) = cache.get(path) {
        return handle.clone();
    }
    let wz = wz::WzData::global();
    let dynamic_image = wz.load_image(path).unwrap_or_else(|e| {
        panic!("failed to load image at {path}: {e}")
    });
    let rgba = dynamic_image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let image = Image::new(
        Extent3d { width, height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        rgba.into_raw(),
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let handle = images.add(image);
    cache.insert(path.to_string(), handle.clone());
    handle
}

fn spawn_tile_entity(
    tile: &wz::TilePlacement,
    commands: &mut Commands,
    z: f32,
    images: &mut HashMap<String, Handle<Image>>,
    assets: &mut Assets<Image>,
) -> Entity {
    let pos = Vec2::new(tile.pos.0, tile.pos.1);
    let origin = Vec2::new(tile.origin.0, tile.origin.1);
    let base = pos - origin;
    let handle = load_map_image(&tile.image_path, images, assets);
    let mut entity = commands.spawn((
        Name::new(format!("Tile({})", tile.z)),
        Sprite::from_image(handle),
        Transform::from_translation(base.extend(z)),
        MapSprite,
    ));

    if !tile.animation_frames.is_empty() {
        let frames: Vec<MapAnimFrame> = tile.animation_frames.iter().map(|f| {
            let origin = Vec2::new(f.origin.0, f.origin.1);
            let handle = load_map_image(&f.image_path, images, assets);
            MapAnimFrame { image: handle, origin, delay: f.delay }
        }).collect();
        let delay = frames[0].delay.max(50);
        entity.insert(MapAnimator {
            frames,
            current: 0,
            timer: Timer::from_seconds(delay as f32 / 1000.0, TimerMode::Repeating),
            pos: base,
            flip: false,
        });
    }

    entity.id()
}

fn spawn_obj_entity(
    obj: &wz::ObjPlacement,
    commands: &mut Commands,
    z: f32,
    images: &mut HashMap<String, Handle<Image>>,
    assets: &mut Assets<Image>,
) -> Entity {
    let pos = Vec2::new(obj.pos.0, obj.pos.1);
    let origin = Vec2::new(obj.origin.0, obj.origin.1);
    let base = pos - origin;
    let handle = load_map_image(&obj.image_path, images, assets);
    let mut entity = commands.spawn((
        Name::new(format!("Obj({})", obj.z)),
        Sprite {
            image: handle,
            flip_x: obj.flip,
            ..default()
        },
        Transform::from_translation(base.extend(z)),
        MapSprite,
    ));

    if !obj.animation_frames.is_empty() {
        let frames: Vec<MapAnimFrame> = obj.animation_frames.iter().map(|f| {
            let origin = Vec2::new(f.origin.0, f.origin.1);
            let handle = load_map_image(&f.image_path, images, assets);
            MapAnimFrame { image: handle, origin, delay: f.delay }
        }).collect();
        let delay = frames[0].delay.max(50);
        entity.insert(MapAnimator {
            frames,
            current: 0,
            timer: Timer::from_seconds(delay as f32 / 1000.0, TimerMode::Repeating),
            pos: base,
            flip: obj.flip,
        });
    }

    if obj.flow != 0 || obj.animation_frames.iter().any(|f| f.move_type != 0) {
        let first = obj.animation_frames.first();
        entity.insert(MapMoveEffect {
            base,
            move_type: first.map(|f| f.move_type).unwrap_or(0),
            move_w: first.map(|f| f.move_w).unwrap_or(0.0),
            move_h: first.map(|f| f.move_h).unwrap_or(0.0),
            move_p: first.map(|f| f.move_p).unwrap_or(6283.0),
            move_r: first.map(|f| f.move_r).unwrap_or(0.0),
            a0: first.map(|f| f.a0).unwrap_or(1.0),
            a1: first.map(|f| f.a1).unwrap_or(1.0),
            flow: obj.flow,
            rx: obj.rx,
            ry: obj.ry,
            cx: obj.cx,
            cy: obj.cy,
        });
    }

    entity.id()
}

fn spawn_background_entity(
    bg: &wz::BackgroundData,
    commands: &mut Commands,
    z: f32,
    viewport: Vec2,
    tex_size: Vec2,
    images: &mut HashMap<String, Handle<Image>>,
    assets: &mut Assets<Image>,
) -> Vec<Entity> {
    let pos = Vec2::new(bg.pos.0, bg.pos.1);
    let origin = Vec2::new(bg.origin.0, bg.origin.1);
    let handle = load_map_image(&bg.image_path, images, assets);

    let tile_x = matches!(bg.btype, 1 | 3 | 4 | 6 | 7);
    let tile_y = matches!(bg.btype, 2 | 3 | 5 | 6 | 7);

    if !tile_x && !tile_y {
        let mut entity = commands.spawn((
            Name::new(format!("Bg({})", bg.index)),
            Sprite {
                image: handle,
                flip_x: bg.flip,
                ..default()
            },
            Transform::from_translation((pos - origin).round().extend(z)),
            MapSprite,
        ));

        insert_background_motion(&mut entity, pos, origin, bg.rx, bg.ry, bg.btype);

        if !bg.animation_frames.is_empty() {
            let frames: Vec<MapAnimFrame> = bg.animation_frames.iter().map(|f| {
                let origin = Vec2::new(f.origin.0, f.origin.1);
                let handle = load_map_image(&f.image_path, images, assets);
                MapAnimFrame { image: handle, origin, delay: f.delay }
            }).collect();
            let delay = frames[0].delay.max(50);
            entity.insert(MapAnimator {
                frames,
                current: 0,
                timer: Timer::from_seconds(delay as f32 / 1000.0, TimerMode::Repeating),
                pos,
                flip: bg.flip,
            });
        }

        return vec![entity.id()];
    }

    let spacing_x: f32 = if bg.cx == 0 {
        tex_size.x
    } else {
        bg.cx.unsigned_abs() as f32
    };
    let spacing_y: f32 = if bg.cy == 0 {
        tex_size.y
    } else {
        bg.cy.unsigned_abs() as f32
    };

    let num_cols = if tile_x {
        (viewport.x / spacing_x).ceil() as i32 + 2
    } else {
        1
    };
    let num_rows = if tile_y {
        (viewport.y / spacing_y).ceil() as i32 + 2
    } else {
        1
    };

    let mut entities = Vec::with_capacity((num_cols * num_rows) as usize);
    for row in 0..num_rows {
        for col in 0..num_cols {
            let t = Vec2::new(col as f32 * spacing_x, row as f32 * spacing_y);

            let mut entity = commands.spawn((
                Name::new(format!("BgTile({})", bg.index)),
                Sprite {
                    image: handle.clone(),
                    flip_x: bg.flip,
                    ..default()
                },
                Transform::from_translation(Vec2::from(pos - origin + t).extend(z)),
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

            insert_background_motion(&mut entity, pos, origin, bg.rx, bg.ry, bg.btype);

            if !bg.animation_frames.is_empty() {
                let frames: Vec<MapAnimFrame> = bg.animation_frames.iter().map(|f| {
                    let origin = Vec2::new(f.origin.0, f.origin.1);
                    let handle = load_map_image(&f.image_path, images, assets);
                    MapAnimFrame { image: handle, origin, delay: f.delay }
                }).collect();
                let delay = frames[0].delay.max(50);
                entity.insert(MapAnimator {
                    frames,
                    current: 0,
                    timer: Timer::from_seconds(delay as f32 / 1000.0, TimerMode::Repeating),
                    pos,
                    flip: bg.flip,
                });
            }

            entities.push(entity.id());
        }
    }

    entities
}

fn insert_background_motion(entity: &mut EntityCommands, pos: Vec2, origin: Vec2, rx: i32, ry: i32, btype: i32) {
    entity.insert(BackgroundMotion {
        pos,
        origin,
        rx,
        ry,
    });

    match btype {
        1 => entity.insert(HorizontalTiledParallaxBackground),
        2 => entity.insert(VerticalTiledParallaxBackground),
        3 => entity.insert(FullyTiledParallaxBackground),
        4 => entity.insert(HorizontalScrollingBackground),
        5 => entity.insert(VerticalScrollingBackground),
        6 => entity.insert(FullyTiledHorizontalScrollingBackground),
        7 => entity.insert(FullyTiledVerticalScrollingBackground),
        _ => entity.insert(ParallaxBackground),
    };
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
