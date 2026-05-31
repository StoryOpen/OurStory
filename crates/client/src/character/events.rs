use bevy::prelude::*;

use crate::character::components::CharacterConfig;

#[derive(Event)]
pub struct SpawnCharacter {
    pub transform: Transform,
    pub config: CharacterConfig,
    pub action: String,
    pub face_expression: String,
}

#[derive(Event)]
pub struct SetAction {
    pub entity: Entity,
    pub action: String,
}
