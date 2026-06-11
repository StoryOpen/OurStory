pub mod animation;
pub mod asset;
pub mod events;

use bevy::asset::AssetServer;
use bevy::prelude::*;

use asset::WzMobAsset;
use crate::GameSet;

pub struct MobPlugin {
    pub cache_capacity: usize,
}

impl Default for MobPlugin {
    fn default() -> Self {
        Self { cache_capacity: 50 }
    }
}

impl Plugin for MobPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<WzMobAsset>()
            .init_asset_loader::<asset::WzMobLoader>()
            .insert_resource(MobAssetRegistry::new(self.cache_capacity))
            .insert_resource(PendingSpawns::default())
            .register_type::<MobId>()
            .add_observer(on_debug_mob_action)
            .add_systems(
                Update,
                (
                    animation::tick_mob_animations,
                    animation::process_pending_spawns,
                )
                    .in_set(GameSet::Animation),
            )
            .add_observer(animation::spawn_mob)
            .add_observer(animation::handle_switch_action);
    }
}

fn on_debug_mob_action(trigger: On<crate::input::ActionEvent>, mut commands: Commands) {
    use crate::input::KeyAction;
    let mob_action = match trigger.event().0 {
        KeyAction::DebugMobStand => "stand",
        KeyAction::DebugMobMove => "move",
        KeyAction::DebugMobHit1 => "hit1",
        KeyAction::DebugMobDie1 => "die1",
        _ => return,
    };
    commands.trigger(events::SwitchMobAction {
        mob_id: 100100,
        action: mob_action.to_string(),
    });
    bevy::log::info!("switch Snail to {mob_action}");
}

#[derive(Default, Resource)]
pub struct PendingSpawns(pub Vec<events::SpawnMob>);

#[derive(Resource)]
pub struct MobAssetRegistry {
    entries: Vec<(i32, Handle<WzMobAsset>)>,
    capacity: usize,
}

impl MobAssetRegistry {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn get_or_load(&mut self, mob_id: i32, asset_server: &AssetServer) -> Handle<WzMobAsset> {
        if let Some(pos) = self.entries.iter().position(|(id, _)| *id == mob_id) {
            let (_, handle) = self.entries.remove(pos);
            self.entries.push((mob_id, handle.clone()));
            return handle;
        }
        let path = format!("wz://Mob/{:07}.img.mob", mob_id);
        let handle = asset_server.load::<WzMobAsset>(&path);
        self.entries.push((mob_id, handle.clone()));
        if self.entries.len() > self.capacity {
            self.entries.remove(0);
        }
        handle
    }

    pub fn peek(&self, mob_id: &i32) -> Option<&Handle<WzMobAsset>> {
        self.entries
            .iter()
            .find(|(id, _)| id == mob_id)
            .map(|(_, h)| h)
    }
}

#[derive(Component, Reflect)]
pub struct MobId(pub i32);

#[derive(Component)]
pub struct MobAnimator {
    pub action: String,
    pub frame: usize,
    pub timer: Timer,
    pub base_x: f32,
    pub base_y: f32,
}
