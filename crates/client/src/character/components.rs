use bevy::prelude::*;

use crate::character::job::Job;

/// Stores the WZ part name on each part entity for reverse lookup during animation.
#[derive(Component)]
pub struct PartName(pub String);

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct CharacterRoot;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct CharacterBody;

#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct CharacterConfig {
    pub skin_suffix: u32,
    pub hair_id: u32,
    pub face_id: u32,
    pub job: Job,
    pub equipment: Vec<(crate::character::types::EquipSlot, u32)>,
}

/// Pre-computed pose for one part in one frame.
#[derive(Clone, Reflect)]
pub struct PartPose {
    pub image: Handle<Image>,
    pub position: Vec3,
    pub anchor: Vec2,
    pub visible: bool,
}

impl PartPose {
    pub fn hidden() -> Self {
        PartPose {
            image: Handle::default(),
            position: Vec3::ZERO,
            anchor: Vec2::ZERO,
            visible: false,
        }
    }
}

/// Shared action frame data. Stored on the root entity, read by the animation system.
#[derive(Component)]
pub struct CurrentAction {
    pub frames: Vec<crate::character::systems::ActionFrame>,
}

/// Face animation state, stored on the root entity.
/// Updated independently from body animation.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct CharacterFaceAnimation {
    pub expression: String,
    pub frame_idx: usize,
    pub timer: Timer,
    pub frames: Vec<FaceFrame>,
    pub face_entity: Option<Entity>,
}

/// One frame of a face expression.
#[derive(Clone, Reflect)]
pub struct FaceFrame {
    pub image: Handle<Image>,
    pub anchor: Vec2,
    pub delay_ms: u32,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct CharacterActionAnimation {
    pub action: String,
    pub default_action: String,
    pub return_to_default: bool,
    pub pending_action: Option<PendingCharacterAction>,
    pub frame_idx: usize,
    pub timer: Timer,
    pub facing_left: bool,
    pub frame_count: usize,
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

/// Stores the current label text for gizmo rendering.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct CharacterLabels {
    pub action: String,
    pub job: String,
}

