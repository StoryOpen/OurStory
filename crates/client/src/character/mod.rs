pub mod components;
pub mod events;
pub mod loader;
pub mod systems;
pub mod types;

use bevy::prelude::*;

use self::loader::WzSpriteCache;
use self::systems::*;
use self::types::{load_smap, load_zmap, SlotMap, ZMap};
use crate::wz::get_cached_base;

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        let base = get_cached_base();
        app.insert_resource(load_zmap(base))
           .insert_resource(load_smap(base))
           .init_resource::<WzSpriteCache>()
           .add_observer(spawn_character)
           .add_observer(on_set_action)
           .add_systems(Update, (animate_characters, character_action_controls));
    }
}
