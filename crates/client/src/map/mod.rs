pub mod components;
pub mod events;
pub mod resources;
pub mod systems;

use crate::wz::asset_loaders::{WzMapAsset, WzMapLoader};
use crate::GameSet;
use bevy::prelude::*;

pub struct MapPlugin {
    pub start_map: Option<String>,
    pub cache_capacity: usize,
}

impl Default for MapPlugin {
    fn default() -> Self {
        Self {
            start_map: Some("Map/Map/Map1/100010000.img".into()),
            cache_capacity: 5,
        }
    }
}

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<WzMapAsset>()
            .init_asset_loader::<WzMapLoader>()
            .insert_resource(resources::MapCache::new(self.cache_capacity))
            .insert_resource(resources::CurrentMap(resources::MapState::None))
            .add_systems(
                Update,
                (
                    systems::on_asset_loaded,
                    systems::tick_map_animations,
                    systems::tick_move_effects,
                    (
                        systems::tick_parallax_backgrounds,
                        systems::tick_horizontal_tiled_parallax_backgrounds,
                        systems::tick_vertical_tiled_parallax_backgrounds,
                        systems::tick_fully_tiled_parallax_backgrounds,
                        systems::tick_horizontal_scrolling_backgrounds,
                        systems::tick_vertical_scrolling_backgrounds,
                        systems::tick_fully_tiled_horizontal_scrolling_backgrounds,
                        systems::tick_fully_tiled_vertical_scrolling_backgrounds,
                    )
                        .in_set(GameSet::Visuals),
                ),
            )
            .add_observer(systems::handle_request_map)
            .add_observer(systems::spawn_map);

        if let Some(path) = &self.start_map {
            let p = path.clone();
            app.add_systems(Startup, move |mut commands: Commands| {
                commands.trigger(events::RequestMap(p.clone()));
            });
        }
    }
}
