use super::components::*;
use super::events::*;
use super::resources::*;
use crate::layer::GameLayer;
use crate::physics::FootholdGraph;
use super::asset_loader::{BackgroundData, ObjData, TileData, WzMapAsset};
use bevy::{asset::AssetEvent, ecs::message::MessageReader, prelude::*};

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

    let asset_path = format!("wz://{}.map", path);
    let handle = asset_server.load::<WzMapAsset>(&asset_path);
    *current_map = CurrentMap(MapState::Loading {
        path: path.clone(),
        handle,
    });
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
                commands.trigger(MapReady {
                    path: loading_path,
                    handle: loading_handle,
                });
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
    commands.trigger(super::events::MapLoaded {
        path: ev.path.clone(),
        bounds,
    });

    let viewport = window
        .single()
        .map(|w| Vec2::new(w.width(), w.height()))
        .unwrap_or(Vec2::new(1920.0, 1080.0));

    let mut sprites = Vec::new();

    for b in asset.backgrounds.iter().take(1) {
        info!("{:?}", b);
        info!("{:?}", b.image);
        let tex_size = images.get(&b.image).map(|i| i.size_f32()).unwrap();
        let z = if b.front {
            GameLayer::Foreground.with_offset(b.index as f32)
        } else {
            GameLayer::Background.with_offset(b.index as f32)
        };
        let mut ents = spawn_background_entity(b, &mut commands, z, viewport, tex_size);
        sprites.append(&mut ents);
    }

    // for tile in asset.tiles.iter() {
    //     let layer_offset = tile.layer as f32 * 100.0 + tile.z as f32 + tile.zid as f32 * 0.001;
    //     let z = GameLayer::Tile.with_offset(layer_offset);
    //     sprites.push(spawn_tile_entity(tile, &mut commands, z));
    // }

    // for obj in asset.objs.iter() {
    //     let layer_offset = obj.layer as f32 * 100.0 + obj.z as f32 + obj.zid as f32 * 0.001;
    //     let z = if obj.z < 0 {
    //         GameLayer::ObjBehind.with_offset(layer_offset)
    //     } else {
    //         GameLayer::ObjFront.with_offset(layer_offset)
    //     };
    //     sprites.push(spawn_obj_entity(obj, &mut commands, z));
    // }

    info!("spawned {} sprites for map {}", sprites.len(), ev.path);
    *current_map = CurrentMap(MapState::Loaded {
        path: ev.path.clone(),
        sprites,
        handle: ev.handle.clone(),
    });
}

fn compute_bounds(
    info: &super::asset_loader::MapInfo,
    footholds: &[crate::wz::foothold::Foothold],
) -> super::resources::MapBounds {
    if let (Some(l), Some(r), Some(t), Some(b)) =
        (info.vr_left, info.vr_right, info.vr_top, info.vr_bottom)
    {
        super::resources::MapBounds::from_vr(l, r, t, b)
    } else if !footholds.is_empty() {
        super::resources::MapBounds::from_footholds(footholds)
    } else {
        super::resources::MapBounds {
            left: -1000.0,
            right: 1000.0,
            top: 1000.0,
            bottom: -1000.0,
        }
    }
}

fn spawn_tile_entity(tile: &TileData, commands: &mut Commands, z: f32) -> Entity {
    let base = tile.pos - tile.origin;
    let mut entity = commands.spawn((
        Sprite::from_image(tile.image.clone()),
        Transform::from_translation(base.extend(z)),
        MapSprite,
    ));

    if !tile.animation_frames.is_empty() {
        let delay = tile.animation_frames[0].delay.max(50);
        entity.insert(MapAnimator {
            frames: tile.animation_frames.clone(),
            current: 0,
            timer: Timer::from_seconds(delay as f32 / 1000.0, TimerMode::Repeating),
            base,
            flip: false,
        });
    }

    entity.id()
}

fn spawn_obj_entity(obj: &ObjData, commands: &mut Commands, z: f32) -> Entity {
    let base = obj.pos - obj.origin;
    let mut entity = commands.spawn((
        Sprite {
            image: obj.image.clone(),
            flip_x: obj.flip,
            ..default()
        },
        Transform::from_translation(base.extend(z)),
        MapSprite,
    ));

    if !obj.animation_frames.is_empty() {
        let delay = obj.animation_frames[0].delay.max(50);
        entity.insert(MapAnimator {
            frames: obj.animation_frames.clone(),
            current: 0,
            timer: Timer::from_seconds(delay as f32 / 1000.0, TimerMode::Repeating),
            base,
            flip: obj.flip,
        });
    }

    if obj.flow != 0 || obj.animation_frames.iter().any(|f| f.move_type != 0) {
        let move_type = obj
            .animation_frames
            .first()
            .map(|f| f.move_type)
            .unwrap_or(0);
        let move_w = obj
            .animation_frames
            .first()
            .map(|f| f.move_w)
            .unwrap_or(0.0);
        let move_h = obj
            .animation_frames
            .first()
            .map(|f| f.move_h)
            .unwrap_or(0.0);
        let move_p = obj
            .animation_frames
            .first()
            .map(|f| f.move_p)
            .unwrap_or(6283.0);
        let move_r = obj
            .animation_frames
            .first()
            .map(|f| f.move_r)
            .unwrap_or(0.0);
        let a0 = obj.animation_frames.first().map(|f| f.a0).unwrap_or(1.0);
        let a1 = obj.animation_frames.first().map(|f| f.a1).unwrap_or(1.0);

        entity.insert(MapMoveEffect {
            base,
            move_type,
            move_w,
            move_h,
            move_p,
            move_r,
            a0,
            a1,
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
    b: &BackgroundData,
    commands: &mut Commands,
    z: f32,
    viewport: Vec2,
    tex_size: Vec2,
) -> Vec<Entity> {
    let tile_x = matches!(b.btype, 1 | 3 | 4 | 6 | 7);
    let tile_y = matches!(b.btype, 2 | 3 | 5 | 6 | 7);

    if !tile_x && !tile_y {
        let mut entity = commands.spawn((
            Sprite {
                image: b.image.clone(),
                flip_x: b.flip,
                ..default()
            },
            Transform::from_translation((b.pos - b.origin).extend(z)),
            MapSprite,
        ));

        insert_background_motion(&mut entity, b);

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

    let spacing_x: f32 = if b.cx == 0 {
        tex_size.x
    } else {
        b.cx.unsigned_abs() as f32
    };
    let spacing_y: f32 = if b.cy == 0 {
        tex_size.y
    } else {
        b.cy.unsigned_abs() as f32
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
                Transform::from_translation(Vec3::new(center.x + tx, center.y + ty, z)),
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

            insert_background_motion(&mut entity, b);

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

fn insert_background_motion(entity: &mut EntityCommands, b: &BackgroundData) {
    entity.insert(BackgroundMotion {
        pos: b.pos,
        origin: b.origin,
        rx: b.rx,
        ry: b.ry,
    });

    match b.btype {
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
    mut query: Query<(
        &mut MapAnimator,
        &mut Sprite,
        &mut Transform,
        Option<&BackgroundMotion>,
    )>,
) {
    for (mut anim, mut sprite, mut transform, background) in &mut query {
        anim.timer.tick(time.delta());
        if !anim.timer.just_finished() {
            continue;
        }

        anim.current = (anim.current + 1) % anim.frames.len();
        let frame = &anim.frames[anim.current];

        sprite.image = frame.image.clone();
        sprite.flip_x = anim.flip;

        // Background positions are managed by dedicated background systems.
        if background.is_none() {
            transform.translation = (anim.base - frame.origin).extend(transform.translation.z);
        }

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
            -(bg.ry as f32) * cam_pos.y / 100.0,
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
            bg.rx as f32 * cam_pos.x / 100.0,
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
            -(bg.ry as f32) * cam_pos.y / 100.0,
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
            bg.rx as f32 * cam_pos.x / 100.0,
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
        rx as f32 * cam_pos.x / 100.0,
        -(ry as f32) * cam_pos.y / 100.0,
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
