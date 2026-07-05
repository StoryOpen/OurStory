pub mod animation;
pub mod events;

use bevy::prelude::*;

use crate::wz::asset_loaders::WzNpcAsset;
use crate::GameSet;

pub struct NpcPlugin {
    pub cache_capacity: usize,
}

impl Default for NpcPlugin {
    fn default() -> Self {
        Self { cache_capacity: 50 }
    }
}

impl Plugin for NpcPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<WzNpcAsset>()
            .init_asset_loader::<crate::wz::asset_loaders::WzNpcLoader>()
            .insert_resource(NpcAssetRegistry::new(self.cache_capacity))
            .insert_resource(PendingNpcSpawns::default())
            .register_type::<NpcId>()
            .add_systems(
                Update,
                (
                    animation::tick_npc_animations,
                    animation::process_pending_spawns,
                )
                    .in_set(GameSet::Animation),
            )
            .add_observer(animation::spawn_npc);
    }
}

#[derive(Resource)]
pub struct NpcAssetRegistry {
    entries: Vec<(i32, Handle<WzNpcAsset>)>,
    capacity: usize,
}

impl NpcAssetRegistry {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn get_or_load(
        &mut self,
        npc_id: i32,
        asset_server: &AssetServer,
    ) -> Handle<WzNpcAsset> {
        if let Some(pos) = self.entries.iter().position(|(id, _)| *id == npc_id) {
            let (_, handle) = self.entries.remove(pos);
            self.entries.push((npc_id, handle.clone()));
            return handle;
        }
        let path = format!("wz://Npc/{:07}.img.npc", npc_id);
        let handle = asset_server.load::<WzNpcAsset>(&path);
        self.entries.push((npc_id, handle.clone()));
        if self.entries.len() > self.capacity {
            self.entries.remove(0);
        }
        handle
    }

    pub fn peek(&self, npc_id: &i32) -> Option<&Handle<WzNpcAsset>> {
        self.entries
            .iter()
            .find(|(id, _)| id == npc_id)
            .map(|(_, h)| h)
    }
}

#[derive(Default, Resource)]
pub struct PendingNpcSpawns(pub Vec<events::SpawnNpc>);

#[derive(Component, Reflect)]
pub struct NpcId(pub i32);

#[derive(Component)]
pub struct NpcAnimator {
    pub action: String,
    pub frame: usize,
    pub timer: Timer,
    pub base_x: f32,
    pub base_y: f32,
}
