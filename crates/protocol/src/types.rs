use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub x: i16,
    pub y: i16,
}

impl Position {
    pub const fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Job {
    Beginner,
    Warrior,
    Mage,
    Bowman,
    Thief,
    Pirate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct MapId(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct PlayerId(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct MobId(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct ChannelId(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct WorldId(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct QuestId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestState {
    NotStarted = 0,
    Started = 1,
    Completed = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectiveType {
    Kill,
    Item,
    Npc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveInfo {
    pub obj_type: ObjectiveType,
    pub target_id: u32,
    pub current: u32,
    pub target: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemGrant {
    pub item_id: u32,
    pub count: i32,
    pub period_minutes: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopReason {
    Mob,
    Item,
    Npc,
    Quest,
    Generic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DialogBranch {
    Yes { pages: u32 },
    No,
    Stop { reason: StopReason },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestDialog {
    pub quest_id: u32,
    pub stage: u8,
    pub branch: DialogBranch,
}
