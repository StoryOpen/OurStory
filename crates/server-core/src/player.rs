use protocol::types::{Job, MapId, PlayerId, Position, WorldId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub job: Job,
    pub level: i16,
    pub map_id: MapId,
    pub position: Position,
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub exp: i64,
    pub meso: i64,
    pub world_id: WorldId,
    pub buffs: Vec<Buff>,
    pub mount: Option<Mount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Buff {
    pub skill_id: i32,
    pub remaining_ms: i32,
}

#[derive(Debug, Clone)]
pub struct Mount {
    pub mount_id: i32,
    pub level: i8,
    pub exp: i32,
    pub tiredness: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSnapshot {
    pub id: PlayerId,
    pub hp: i32,
    pub mp: i32,
    pub position: Position,
    pub map_id: MapId,
    pub buffs: Vec<Buff>,
}

impl Player {
    pub fn snapshot(&self) -> PlayerSnapshot {
        PlayerSnapshot {
            id: self.id,
            hp: self.hp,
            mp: self.mp,
            position: self.position,
            map_id: self.map_id,
            buffs: self.buffs.clone(),
        }
    }
}
