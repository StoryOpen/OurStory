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

#[derive(Event)]
pub struct SetFlip {
    pub entity: Entity,
    pub facing_left: bool,
}

#[derive(Event)]
pub struct UseSkill {
    pub entity: Entity,
    pub skill_id: u32,
}
