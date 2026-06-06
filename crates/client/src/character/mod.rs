pub mod components;
pub mod events;
pub mod loader;
pub mod systems;
pub mod types;

use bevy::prelude::*;

use self::loader::WzSpriteCache;
use self::systems::*;
use self::types::{load_smap, load_zmap};
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
            .add_observer(on_character_action)
            .add_systems(Update, (animate_characters,))
            .add_systems(Startup, spawn_test_character);
    }
}

fn spawn_test_character(mut commands: Commands) {
    use crate::character::{components::CharacterConfig, events::SpawnCharacter, types::EquipSlot};
    commands.trigger(SpawnCharacter {
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        config: CharacterConfig {
            skin_suffix: 2000,
            hair_id: 31200,
            face_id: 21405,
            equipment: vec![
                (EquipSlot::Cap, 01002419),
                (EquipSlot::Coat, 01042013),
                (EquipSlot::Pants, 01060135),
                (EquipSlot::Shoes, 01072306),
                (EquipSlot::Glove, 01082178),
                (EquipSlot::Weapon, 01452000),
                (EquipSlot::Shield, 01092027),
                (EquipSlot::Cape, 01102149),
            ],
        },
        action: "stand1".into(),
        face_expression: "default".into(),
    });
}
