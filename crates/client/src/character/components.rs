use std::collections::HashMap;
use bevy::prelude::*;

use crate::character::types::FrameData;

#[derive(Component)]
pub struct CharacterRoot;

#[derive(Component, Clone)]
pub struct CharacterConfig {
    pub skin_suffix: u32,
    pub hair_id: u32,
    pub face_id: u32,
    pub equipment: Vec<(crate::character::types::EquipSlot, u32)>,
}

#[derive(Component)]
pub struct CharacterAnimation {
    pub action: String,
    pub frame_idx: usize,
    pub timer: Timer,
    pub face_expression: String,
    pub face_frame_idx: usize,
    pub face_timer: Timer,
}

#[derive(Component)]
pub struct CharacterFrameData {
    pub actions: HashMap<String, Vec<FrameData>>,
    pub face_expressions: HashMap<String, Vec<FrameData>>,
}

#[derive(Component)]
pub struct PartEntities {
    pub map: HashMap<String, Entity>,
}

#[derive(Component)]
pub struct CharacterPart {
    pub layer: String,
    pub z_base: f32,
}
