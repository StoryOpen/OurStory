use bevy::prelude::*;

use super::asset::WzMobAsset;
use super::events::{SpawnMob, SwitchMobAction};
use super::{MobAnimator, MobAssetRegistry, MobId, PendingSpawns};

pub fn tick_mob_animations(
    time: Res<Time>,
    mut mob_query: Query<(&mut MobAnimator, &mut Sprite, &mut Transform, &MobId)>,
    assets: Res<Assets<WzMobAsset>>,
    registry: Res<MobAssetRegistry>,
) {
    for (mut animator, mut sprite, mut transform, mob_id) in &mut mob_query {
        animator.timer.tick(time.delta());
        if !animator.timer.just_finished() {
            continue;
        }

        let Some(handle) = registry.peek(&mob_id.0) else {
            continue;
        };
        let Some(asset) = assets.get(handle) else {
            continue;
        };
        let Some(action) = asset.actions.get(&animator.action) else {
            continue;
        };

        animator.frame = (animator.frame + 1) % action.frames.len();
        let frame = &action.frames[animator.frame];

        animator.timer = Timer::from_seconds(
            frame.delay as f32 / 1000.0,
            TimerMode::Once,
        );

        if let Some(part) = frame.parts.first() {
            sprite.image = part.image_handle.clone();
            transform.translation.x = animator.base_x - part.origin.x;
            transform.translation.y = animator.base_y - part.origin.y;
        }
    }
}

pub fn process_pending_spawns(
    mut pending: ResMut<PendingSpawns>,
    mut commands: Commands,
    registry: Res<MobAssetRegistry>,
    assets: Res<Assets<WzMobAsset>>,
) {
    pending.0.retain(|ev| {
        let Some(handle) = registry.peek(&ev.mob_id) else {
            return true;
        };
        let Some(asset) = assets.get(handle) else {
            return true;
        };
        spawn_one(&mut commands, ev, asset);
        false
    });
}

pub fn spawn_mob(
    trigger: On<SpawnMob>,
    mut commands: Commands,
    mut pending: ResMut<PendingSpawns>,
    mut registry: ResMut<MobAssetRegistry>,
    asset_server: Res<AssetServer>,
    assets: Res<Assets<WzMobAsset>>,
) {
    let ev = trigger.event();
    let handle = registry.get_or_load(ev.mob_id, &asset_server);

    if let Some(asset) = assets.get(&handle) {
        spawn_one(&mut commands, ev, asset);
    } else {
        pending.0.push(ev.clone());
    }
}

pub fn handle_switch_action(
    trigger: On<SwitchMobAction>,
    mut mob_query: Query<(&mut MobAnimator, &mut Sprite, &mut Transform, &MobId)>,
    assets: Res<Assets<WzMobAsset>>,
    registry: Res<MobAssetRegistry>,
) {
    let ev = trigger.event();
    let Some(handle) = registry.peek(&ev.mob_id) else {
        return;
    };
    let Some(asset) = assets.get(handle) else {
        return;
    };
    if !asset.actions.contains_key(&ev.action) {
        bevy::log::warn!("mob {} has no action '{}'", ev.mob_id, ev.action);
        return;
    }

    for (mut animator, mut sprite, mut transform, mob_id) in &mut mob_query {
        if mob_id.0 != ev.mob_id {
            continue;
        }
        let action = &asset.actions[&ev.action];
        let Some(first_frame) = action.frames.first() else {
            continue;
        };
        let Some(part) = first_frame.parts.first() else {
            continue;
        };

        animator.action = ev.action.clone();
        animator.frame = 0;
        animator.timer = Timer::from_seconds(
            first_frame.delay as f32 / 1000.0,
            TimerMode::Once,
        );
        sprite.image = part.image_handle.clone();
        transform.translation = Vec3::new(
            animator.base_x - part.origin.x,
            animator.base_y - part.origin.y,
            transform.translation.z,
        );
    }
}

fn spawn_one(commands: &mut Commands, ev: &SpawnMob, asset: &WzMobAsset) {
    let action_name = if asset.actions.contains_key("stand") {
        "stand"
    } else {
        match asset.actions.keys().next() {
            Some(k) => k.as_str(),
            None => {
                bevy::log::warn!("mob {} has no actions", ev.mob_id);
                return;
            }
        }
    };

    let Some(action) = asset.actions.get(action_name) else {
        return;
    };

    let Some(first_frame) = action.frames.first() else {
        return;
    };

    let part = match first_frame.parts.first() {
        Some(p) => p,
        None => return,
    };

    commands.spawn((
        MobId(ev.mob_id),
        MobAnimator {
            action: action_name.to_string(),
            frame: 0,
            timer: Timer::from_seconds(
                first_frame.delay as f32 / 1000.0,
                TimerMode::Once,
            ),
            base_x: ev.x,
            base_y: ev.y,
        },
        Sprite::from_image(part.image_handle.clone()),
        Transform::from_xyz(
            ev.x - part.origin.x,
            ev.y - part.origin.y,
            ev.z as f32,
        ),
    ));

    bevy::log::info!("spawned mob {} ({}) at ({}, {}, {})", ev.mob_id, asset.info.name, ev.x, ev.y, ev.z);
}
