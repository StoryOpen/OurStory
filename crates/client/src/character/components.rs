use bevy::prelude::*;
use std::collections::HashMap;

use crate::character::job::Job;
use crate::character::types::{EquipmentEntry, FrameData};

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct CharacterRoot {
    pub body_origin: Vec2,
}

#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct CharacterConfig {
    pub skin_suffix: u32,
    pub hair_id: u32,
    pub face_id: u32,
    pub job: Job,
    pub equipment: Vec<(crate::character::types::EquipSlot, u32)>,
}

#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct CharacterEquipment {
    pub entries: Vec<EquipmentEntry>,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct CharacterAnimation {
    pub action: String,
    pub default_action: String,
    pub return_to_default: bool,
    pub pending_action: Option<PendingCharacterAction>,
    pub frame_idx: usize,
    pub timer: Timer,
    pub face_expression: String,
    pub face_frame_idx: usize,
    pub face_timer: Timer,
    pub facing_left: bool,
}

#[derive(Debug, Clone, Reflect)]
pub enum PendingCharacterAction {
    Action {
        action: String,
        return_to_default: bool,
    },
    Skill {
        skill_id: u32,
    },
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct CharacterFrameData {
    pub actions: HashMap<String, Vec<FrameData>>,
    pub face_expressions: HashMap<String, Vec<FrameData>>,
}

#[derive(Component)]
pub struct PartEntities {
    pub map: HashMap<String, Entity>,
}

#[derive(Component)]
pub struct CharacterActionLabel;

#[derive(Component)]
pub struct CharacterJobLabel;

#[derive(Component)]
pub struct SkillNameLabel;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct CharacterPart {
    pub layer: String,
    pub z_base: f32,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct CharacterLayer(pub u8);
