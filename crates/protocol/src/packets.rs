use crate::types::Position;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerListRequest;

#[derive(Debug, Serialize, Deserialize)]
pub struct CharacterSelect {
    pub character_id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangeMapRequest {
    pub target_map_id: i32,
    pub portal: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangeChannelRequest {
    pub target_channel: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerMove {
    pub position: Position,
    pub moves: Vec<MoveAction>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MoveAction {
    pub kind: MoveKind,
    pub x: i16,
    pub y: i16,
    pub stance: i8,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MoveKind {
    Walk,
    Jump,
    Fall,
    Stand,
    Sit,
    Fly,
}
