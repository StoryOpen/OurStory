use bevy::prelude::*;

use crate::character::components::CharacterConfig;

#[derive(Event)]
pub struct SpawnCharacter {
    pub transform: Transform,
    pub config: CharacterConfig,
    pub action: String,
    pub face_expression: String,
}

#[derive(EntityEvent)]
pub struct SetAction {
    pub entity: Entity,
    pub action: String,
    pub return_to_default: bool,
}

#[derive(EntityEvent)]
pub struct SetFaceExpression {
    pub entity: Entity,
    pub expression: String,
    pub action: String,
}

#[derive(EntityEvent)]
pub struct SetFacing {
    pub entity: Entity,
    pub facing_left: bool,
}

#[derive(EntityEvent)]
pub struct UseSkill {
    pub entity: Entity,
    pub skill_id: u32,
}
