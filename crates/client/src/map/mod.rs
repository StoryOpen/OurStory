pub mod events;
pub mod resources;
pub mod systems;

use bevy::prelude::*;
use crate::wz::asset_loader::{WzMapAsset, WzMapLoader};

pub struct MapPlugin {
    pub default_map: Option<String>,
    pub cache_capacity: usize,
}

impl Default for MapPlugin {
    fn default() -> Self {
        Self { default_map: Some("Map/Map/Map1/100000000.img".into()), cache_capacity: 5 }
    }
}

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_asset::<WzMapAsset>()
            .init_asset_loader::<WzMapLoader>()
            .insert_resource(resources::MapCache::new(self.cache_capacity))
            .insert_resource(resources::CurrentMap(resources::MapState::None))
            .add_systems(Update, systems::on_asset_loaded)
            .add_observer(systems::handle_request_map)
            .add_observer(systems::spawn_map);

        if let Some(path) = &self.default_map {
            let p = path.clone();
            app.add_systems(Startup, move |mut commands: Commands| {
                commands.trigger(events::RequestMap(p.clone()));
            });
        }
    }
}
