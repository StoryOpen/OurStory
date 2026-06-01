use crate::types::{ItemGrant, ObjectiveInfo, Position, QuestDialog};
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

// === Quest packets ===

#[derive(Debug, Serialize, Deserialize)]
pub struct NpcQuestRequest {
    pub npc_id: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NpcQuestList {
    pub npc_id: u32,
    pub startable: Vec<u32>,
    pub completable: Vec<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuestStartRequest {
    pub quest_id: u32,
    pub accept: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuestStarted {
    pub quest_id: u32,
    pub objectives: Vec<ObjectiveInfo>,
    pub dialog: QuestDialog,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuestCompleteRequest {
    pub quest_id: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuestCompleted {
    pub quest_id: u32,
    pub exp: u32,
    pub items: Vec<ItemGrant>,
    pub next_quest: Option<u32>,
    pub dialog: QuestDialog,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuestProgress {
    pub quest_id: u32,
    pub objective_idx: u32,
    pub current: u32,
    pub target: u32,
    pub completable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuestForfeitRequest {
    pub quest_id: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuestForfeited {
    pub quest_id: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActiveQuestState {
    pub quest_id: u32,
    pub objectives: Vec<ObjectiveInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuestSync {
    pub active: Vec<ActiveQuestState>,
    pub completed_count: u32,
}
