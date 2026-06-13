use bevy::prelude::*;

use crate::wz::asset_loaders::WzNpcAsset;
use super::events::SpawnNpc;
use super::{NpcAnimator, NpcAssetRegistry, NpcId, PendingNpcSpawns};
use crate::layer::GameLayer;
use crate::wz::WzImageCache;

pub fn tick_npc_animations(
    time: Res<Time>,
    mut npc_query: Query<(&mut NpcAnimator, &mut Sprite, &mut Transform, &NpcId)>,
    assets: Res<Assets<WzNpcAsset>>,
    registry: Res<NpcAssetRegistry>,
    mut image_cache: ResMut<WzImageCache>,
    mut images: ResMut<Assets<Image>>,
) {
    for (mut animator, mut sprite, mut transform, npc_id) in &mut npc_query {
        animator.timer.tick(time.delta());
        if !animator.timer.just_finished() {
            continue;
        }

        let Some(handle) = registry.peek(&npc_id.0) else {
            continue;
        };
        let Some(asset) = assets.get(handle) else {
            continue;
        };
        let Some(action) = asset.0.actions.get(&animator.action) else {
            continue;
        };

        animator.frame = (animator.frame + 1) % action.frames.len();
        let frame = &action.frames[animator.frame];

        animator.timer =
            Timer::from_seconds(frame.delay as f32 / 1000.0, TimerMode::Once);

        sprite.image = image_cache.get_or_load(&frame.image_path, &mut images);
        transform.translation.x = animator.base_x - frame.origin.0;
        transform.translation.y = animator.base_y - frame.origin.1;
    }
}

pub fn process_pending_spawns(
    mut pending: ResMut<PendingNpcSpawns>,
    mut commands: Commands,
    registry: Res<NpcAssetRegistry>,
    assets: Res<Assets<WzNpcAsset>>,
    mut image_cache: ResMut<WzImageCache>,
    mut images: ResMut<Assets<Image>>,
) {
    pending.0.retain(|ev| {
        let Some(handle) = registry.peek(&ev.npc_id) else {
            return true;
        };
        let Some(asset) = assets.get(handle) else {
            return true;
        };
        spawn_one(&mut commands, ev, asset, &mut image_cache, &mut images);
        false
    });
}

pub fn spawn_npc(
    trigger: On<SpawnNpc>,
    mut commands: Commands,
    mut pending: ResMut<PendingNpcSpawns>,
    mut registry: ResMut<NpcAssetRegistry>,
    asset_server: Res<AssetServer>,
    assets: Res<Assets<WzNpcAsset>>,
    mut image_cache: ResMut<WzImageCache>,
    mut images: ResMut<Assets<Image>>,
) {
    let ev = trigger.event();
    let handle = registry.get_or_load(ev.npc_id, &asset_server);

    if let Some(asset) = assets.get(&handle) {
        spawn_one(&mut commands, ev, asset, &mut image_cache, &mut images);
    } else {
        pending.0.push(ev.clone());
    }
}

fn spawn_one(
    commands: &mut Commands,
    ev: &SpawnNpc,
    asset: &WzNpcAsset,
    image_cache: &mut WzImageCache,
    images: &mut Assets<Image>,
) {
    let action_name = if asset.0.actions.contains_key("stand") {
        "stand"
    } else {
        match asset.0.actions.keys().next() {
            Some(k) => k.as_str(),
            None => {
                bevy::log::warn!("npc {} has no actions", ev.npc_id);
                return;
            }
        }
    };

    let Some(action) = asset.0.actions.get(action_name) else {
        return;
    };

    let Some(first_frame) = action.frames.first() else {
        return;
    };

    commands.spawn((
        Name::new(format!("Npc({})", ev.npc_id)),
        NpcId(ev.npc_id),
        NpcAnimator {
            action: action_name.to_string(),
            frame: 0,
            timer: Timer::from_seconds(first_frame.delay as f32 / 1000.0, TimerMode::Repeating),
            base_x: ev.x,
            base_y: ev.y,
        },
        Sprite {
            image: image_cache.get_or_load(&first_frame.image_path, images),
            flip_x: ev.flip,
            ..default()
        },
        Transform::from_xyz(
            ev.x - first_frame.origin.0,
            ev.y - first_frame.origin.1,
            GameLayer::Character.with_offset(ev.z as f32),
        ),
    ));

    bevy::log::info!(
        "spawned npc {} at ({}, {}, {})",
        ev.npc_id, ev.x, ev.y, ev.z
    );
}
