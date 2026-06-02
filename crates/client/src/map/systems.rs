use bevy::{
    asset::AssetEvent,
    ecs::message::MessageReader,
    prelude::*,
    sprite::Anchor,
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

    let total = asset.backgrounds.len() + asset.objs.len() + asset.tiles.len();
    let mut sprites = Vec::with_capacity(total);

    for b in &asset.backgrounds {
        if b.front {
            continue;
        }
        sprites.push(spawn_background_entity(b, &mut commands));
    }

    for layer in 0..8u8 {
        for obj in &asset.objs {
            if obj.layer != layer {
                continue;
            }
            sprites.push(spawn_obj_entity(obj, &mut commands));
        }
        for tile in &asset.tiles {
            if tile.layer != layer {
                continue;
            }
            sprites.push(spawn_tile_entity(tile, &mut commands));
        }
    }

    for b in &asset.backgrounds {
        if !b.front {
            continue;
        }
        sprites.push(spawn_background_entity(b, &mut commands));
    }

    info!("spawned {} sprites for map {}", sprites.len(), ev.path);
    *current_map = CurrentMap(MapState::Loaded { path: ev.path.clone(), sprites });
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

fn compute_z(layer: u8, category_offset: i32, z: i32, z_m: i32) -> f32 {
    ((layer as i32) * 100000 + category_offset + z + z_m) as f32
}

fn spawn_tile_entity(tile: &TileData, commands: &mut Commands) -> Entity {
    let z = compute_z(tile.layer, 50000, tile.z, 0);
    let mut entity = commands.spawn((
        Sprite::from_image(tile.image.clone()),
        Anchor::TOP_LEFT,
        Transform::from_xyz(tile.x - tile.origin.x, tile.y - tile.origin.y, z),
        MapSprite,
    ));

    if !tile.animation_frames.is_empty() {
        let delay = tile.animation_frames[0].delay.max(50);
        entity.insert(MapAnimator {
            frames: tile.animation_frames.clone(),
            current: 0,
            timer: Timer::from_seconds(delay as f32 / 1000.0, TimerMode::Repeating),
            base_x: tile.x,
            base_y: tile.y,
            flip: false,
        });
    }

    entity.id()
}

fn spawn_obj_entity(obj: &ObjData, commands: &mut Commands) -> Entity {
    let z = compute_z(obj.layer, 0, obj.z, obj.z_m);
    let mut entity = commands.spawn((
        Sprite {
            image: obj.image.clone(),
            flip_x: obj.flip,
            ..default()
        },
        Anchor::TOP_LEFT,
        Transform::from_xyz(obj.x - obj.origin.x, obj.y - obj.origin.y, z),
        MapSprite,
    ));

    if !obj.animation_frames.is_empty() {
        let delay = obj.animation_frames[0].delay.max(50);
        entity.insert(MapAnimator {
            frames: obj.animation_frames.clone(),
            current: 0,
            timer: Timer::from_seconds(delay as f32 / 1000.0, TimerMode::Repeating),
            base_x: obj.x,
            base_y: obj.y,
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
            base_x: obj.x,
            base_y: obj.y,
            move_type, move_w, move_h, move_p, move_r,
            a0, a1,
            flow: obj.flow,
            rx: obj.rx, ry: obj.ry,
            cx: obj.cx, cy: obj.cy,
        });
    }

    entity.id()
}

fn spawn_background_entity(b: &BackgroundData, commands: &mut Commands) -> Entity {
    let z = if b.front { 300 + b.index } else { -100 - b.index };
    let mut entity = commands.spawn((
        Sprite {
            image: b.image.clone(),
            flip_x: b.flip,
            ..default()
        },
        Anchor::TOP_LEFT,
        Transform::from_xyz(b.x - b.origin.x, b.y - b.origin.y, z as f32),
        MapParallaxBackground {
            base_x: b.x,
            base_y: b.y,
            origin: b.origin,
            rx: b.rx,
            ry: b.ry,
            btype: b.btype,
            cx: b.cx,
            cy: b.cy,
            alpha: b.alpha,
            flip: b.flip,
            front: b.front,
        },
        MapSprite,
    ));

    if !b.animation_frames.is_empty() {
        let delay = b.animation_frames[0].delay.max(50);
        entity.insert(MapAnimator {
            frames: b.animation_frames.clone(),
            current: 0,
            timer: Timer::from_seconds(delay as f32 / 1000.0, TimerMode::Repeating),
            base_x: b.x,
            base_y: b.y,
            flip: b.flip,
        });
    }

    entity.id()
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

        transform.translation.x = anim.base_x - frame.origin.x;
        transform.translation.y = anim.base_y - frame.origin.y;

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

        transform.translation.x = effect.base_x + dx;
        transform.translation.y = effect.base_y + dy;
    }
}

pub fn tick_background_parallax(
    camera: Query<(&Camera, &GlobalTransform)>,
    time: Res<Time>,
    mut backgrounds: Query<(&MapParallaxBackground, &mut Transform, &mut Sprite)>,
) {
    let Ok((_cam, cam_global)) = camera.single() else {
        return;
    };

    let cam_pos = cam_global.translation();
    let elapsed = time.elapsed_secs();

    for (bg, mut transform, mut sprite) in &mut backgrounds {
        let rx = bg.rx as f32;
        let ry = bg.ry as f32;

        let shift_x = rx * (cam_pos.x) / 100.0;
        let shift_y = ry * (cam_pos.y) / 100.0;

        sprite.flip_x = bg.flip;

        let (mut final_x, mut final_y) = match bg.btype {
            0 => (shift_x, shift_y),
            1 => (shift_x, shift_y),
            2 => (shift_x, shift_y),
            3 => (shift_x, shift_y),
            4 => (rx * 5.0 * elapsed - cam_pos.x, shift_y),
            5 => (shift_x, ry * 5.0 * elapsed - cam_pos.y),
            6 => (rx * 5.0 * elapsed - cam_pos.x, shift_y),
            7 => (shift_x, ry * 5.0 * elapsed - cam_pos.y),
            _ => (0.0, 0.0),
        };

        final_x += bg.base_x - bg.origin.x;
        final_y += bg.base_y - bg.origin.y;

        transform.translation.x = final_x;
        transform.translation.y = final_y;
    }
}
