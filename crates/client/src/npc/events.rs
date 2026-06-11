use bevy::prelude::*;

#[derive(Clone, Event)]
pub struct SpawnNpc {
    pub npc_id: i32,
    pub x: f32,
    pub y: f32,
    pub z: i32,
    pub flip: bool,
}
