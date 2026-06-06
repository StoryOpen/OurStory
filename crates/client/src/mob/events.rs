use bevy::prelude::*;

/// Request to spawn a mob entity at the given world position.
///
/// `x` and `y` are Bevy-space (Y-up) world coordinates. If sourcing from
/// WZ map data (e.g. `LifeSpawn`), the values are already converted at the
/// `wz` boundary. If sourcing from network packets (`protocol::Position`,
/// WZ-Y), negate Y at the network handler before triggering.
#[derive(Clone, Event)]
pub struct SpawnMob {
    pub mob_id: i32,
    pub x: f32,
    pub y: f32,
    pub z: i32,
}

#[derive(Event)]
pub struct SwitchMobAction {
    pub mob_id: i32, // target entity mob_id
    pub action: String,
}
