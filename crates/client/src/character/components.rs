use bevy::prelude::*;
use std::collections::HashMap;

use crate::character::types::{EquipmentEntry, FrameData};

#[derive(Component)]
pub struct CharacterRoot;

#[derive(Component, Clone)]
pub struct CharacterConfig {
    pub skin_suffix: u32,
    pub hair_id: u32,
    pub face_id: u32,
    pub equipment: Vec<(crate::character::types::EquipSlot, u32)>,
}

/// Resolved equipment entries with vslot data, populated at spawn. Kept on
/// the entity so vslot-based sprite suppression can be recomputed when
/// equipment changes, and so other systems (stats, tooltip) can read slot
/// ownership without re-parsing the WZ.
#[derive(Component, Clone)]
pub struct CharacterEquipment {
    pub entries: Vec<EquipmentEntry>,
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

#[derive(Component)]
pub struct CharacterLayer(pub u8);
