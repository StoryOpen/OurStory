pub mod components;
pub mod events;
pub mod loader;
pub mod systems;
pub mod types;

use bevy::prelude::*;

use self::loader::WzSpriteCache;
use self::systems::*;

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WzSpriteCache>()
           .add_observer(spawn_character)
           .add_observer(on_set_action)
           .add_systems(Update, (animate_characters, character_action_controls));
    }
}
