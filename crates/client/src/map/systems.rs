use bevy::{
    asset::AssetEvent,
    ecs::message::MessageReader,
    prelude::*,
    sprite::SpriteImageMode,
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
    let mut z = -1000.0;

    // TEMP: only render background 0 for inspection
    if let Some(b) = asset.backgrounds.first() {
        z += 1.0;
        sprites.push(spawn_background_entity(b, &mut commands, z));
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

#[allow(dead_code)]
fn spawn_tile_entity(tile: &TileData, commands: &mut Commands, z: f32) -> Entity {
    let mut entity = commands.spawn((
        Sprite::from_image(tile.image.clone()),
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

#[allow(dead_code)]
fn spawn_obj_entity(obj: &ObjData, commands: &mut Commands, z: f32) -> Entity {
    let mut entity = commands.spawn((
        Sprite {
            image: obj.image.clone(),
            flip_x: obj.flip,
            ..default()
        },
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

fn spawn_background_entity(b: &BackgroundData, commands: &mut Commands, z: f32) -> Entity {
    let tile_x = matches!(b.btype, 1 | 3 | 4 | 6 | 7);
    let tile_y = matches!(b.btype, 2 | 3 | 5 | 6 | 7);

    let mut sprite = Sprite {
        image: b.image.clone(),
        flip_x: b.flip,
        ..default()
    };

    if tile_x || tile_y {
        sprite.image_mode = SpriteImageMode::Tiled { tile_x, tile_y, stretch_value: 1.0 };
    }

    let mut entity = commands.spawn((
        sprite,
        Transform::from_xyz(b.x - b.origin.x, b.y - b.origin.y, z),
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
    window: Query<&Window>,
    time: Res<Time>,
    images: Res<Assets<Image>>,
    mut backgrounds: Query<(&MapParallaxBackground, &mut Transform, &mut Sprite)>,
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

    for (bg, mut transform, mut sprite) in &mut backgrounds {
        let rx = bg.rx as f32;
        let ry = bg.ry as f32;

        sprite.flip_x = bg.flip;

        // Screen-space pivot offset matching NoLifeStory parallax:
        //   shift_x = rx * view_x / 100 + viewport.x / 2     (X: same direction in both systems)
        //   shift_y = ry * view_y / 100 + viewport.y / 2     (Y: view_y = -cam_y, Y-down → Y-up)
        // NoLifeStory uses a biased viewport (300px UI bar, ymax = view_y+250),
        // but our Bevy camera is centered with no UI bar, so we simplify:
        //   shift_y = ry * (-cam_y) / 100 + viewport.y / 2 = -ry * cam_pos.y / 100 + viewport.y / 2
        let shift_x = rx * cam_pos.x / 100.0 + viewport.x / 2.0;
        let shift_y = -ry * cam_pos.y / 100.0 + viewport.y / 2.0;

        // Reference screen-space pivot (dx, dy) matching NoLifeStory's per-type logic
        let (pivot_screen_x, pivot_screen_y) = match bg.btype {
            // Types 0-3: standard parallax (camera-based), optional tiling
            0 | 1 | 2 | 3 => (bg.base_x + shift_x, -bg.base_y + shift_y),
            // Types 4,6: time-based X, camera-based Y, tile X (+ optional tile Y)
            4 | 6 => (
                bg.base_x + rx * 5.0 * elapsed - cam_pos.x,
                -bg.base_y + shift_y,
            ),
            // Types 5,7: camera-based X, time-based Y, tile Y (+ optional tile X)
            5 | 7 => (
                bg.base_x + shift_x,
                -bg.base_y + ry * 5.0 * elapsed + cam_pos.y,
            ),
            _ => (bg.base_x, -bg.base_y),
        };

        // Convert reference screen pivot (Y-down) to Bevy world TOP_LEFT:
        //   Reference draws pivot at screen (dx, dy), so top-left is (dx - ox, dy - oy).
        //   Bevy screen (Y-down) = cam_pos.y + viewport.y/2 - world_y
        //   Bevy world_x = screen_x + cam_pos.x - viewport.x/2
        let mut bevy_x = pivot_screen_x + cam_pos.x - viewport.x / 2.0 - bg.origin.x;
        let mut bevy_y = cam_pos.y + viewport.y / 2.0 - pivot_screen_y + bg.origin.y;

        let tile_x = matches!(bg.btype, 1 | 3 | 4 | 6 | 7);
        let tile_y = matches!(bg.btype, 2 | 3 | 5 | 6 | 7);

        if tile_x || tile_y {
            let tex_size = images.get(&sprite.image).map(|i| i.size_f32());
            let margin = tex_size.unwrap_or(Vec2::splat(2000.0));
            let tiled_size = viewport + margin;
            sprite.custom_size = Some(tiled_size);
            bevy_x -= tiled_size.x / 2.0;
            bevy_y += tiled_size.y / 2.0;

        }

        transform.translation.x = bevy_x;
        transform.translation.y = bevy_y;
    }
}

pub fn draw_background_gizmos(
    mut gizmos: Gizmos,
    images: Res<Assets<Image>>,
    query: Query<(&MapParallaxBackground, &Transform, &Sprite)>,
) {
    for (_bg, transform, sprite) in &query {
        let size = sprite.custom_size.unwrap_or_else(|| {
            images
                .get(&sprite.image)
                .map(|i| i.size_f32())
                .unwrap_or(Vec2::splat(64.0))
        });
        // Anchor::BOTTOM_LEFT: sprite extends right (+X) and up (+Y) from transform
        let center = Vec2::new(
            transform.translation.x + size.x / 2.0,
            transform.translation.y - size.y / 2.0,
        );
        gizmos.rect_2d(
            Isometry2d::new(center, Rot2::default()),
            size,
            Color::srgba(0.0, 1.0, 0.0, 0.5),
        );
    }
}
