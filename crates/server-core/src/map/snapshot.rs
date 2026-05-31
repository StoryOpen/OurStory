use crate::player::Buff;
use protocol::types::{MapId, PlayerId, Position};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSnapshot {
    pub id: PlayerId,
    pub hp: i32,
    pub mp: i32,
    pub position: Position,
    pub map_id: MapId,
    pub buffs: Vec<Buff>,
}
