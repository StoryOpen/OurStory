use bevy::{
    asset::AssetEvent,
    ecs::message::MessageReader,
    prelude::*,
    sprite::Anchor,
};
use crate::wz::asset_loader::WzMapAsset;
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

    let mut sprites = Vec::with_capacity(asset.sprites.len());
    for s in &asset.sprites {
        let e = commands.spawn((
            Sprite::from_image(s.image.clone()),
            Anchor::TOP_LEFT,
            Transform::from_xyz(s.x - s.origin.x, (-s.y) + s.origin.y, s.z as f32),
        )).id();
        sprites.push(e);
    }

    info!("spawned {} sprites for map {}", sprites.len(), ev.path);
    *current_map = CurrentMap(MapState::Loaded { path: ev.path.clone(), sprites });
}
