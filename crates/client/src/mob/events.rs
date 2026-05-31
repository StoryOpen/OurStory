use bevy::prelude::*;

#[derive(Clone, Event)]
pub struct SpawnMob {
    pub mob_id: i32,
    pub x: f32,
    pub y: f32,
    pub z: i32,
}

#[derive(Event)]
pub struct SwitchMobAction {
    pub mob_id: i32,  // target entity mob_id
    pub action: String,
}
