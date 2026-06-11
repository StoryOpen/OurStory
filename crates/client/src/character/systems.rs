use bevy::prelude::*;
use std::collections::HashMap;

use crate::character::components::*;
use crate::character::events::*;
use crate::character::job::Job;
use crate::character::loader::{self, WzSpriteCache};
use crate::character::skills::{LearnedSkills, SkillDatabase, SkillEffect, SkillEffectRoot, SkillType};
use crate::character::types::*;
use crate::input::{CharacterIntent, IsLocalPlayer, KeyAction};
use crate::layer::GameLayer;
use crate::physics::PhysicsState;
use crate::wz::get_cached_base;

/// Tracks the current action index per category for cycling.
#[derive(Resource, Default)]
pub struct CharacterActionCycle {
    pub stance: usize,
    pub alert: usize,
    pub swing: usize,
    pub stab: usize,
    pub multi_swing: usize,
    pub ranged: usize,
    pub magic: usize,
    pub movement_skill: usize,
    pub skill: usize,
}

/// Action lists per category, in the order they appear in WZ.
pub const STANCE_ACTIONS: &[&str] = &[
    "stand1", "stand2", "walk1", "walk2", "sit", "prone", "prone2",
    "ladder", "ladder2", "rope", "rope2", "fly", "float",
];

pub const ALERT_ACTIONS: &[&str] = &[
    "alert", "alert2", "alert3", "alert4", "alert5", "alert6",
];

pub const SWING_ACTIONS: &[&str] = &[
    "swingO1", "swingO2", "swingO3", "swingOF",
    "swingP1", "swingP2", "swingPF",
    "swingT1", "swingT2", "swingT3", "swingTF",
    "swingT2PoleArm", "swingP1PoleArm", "swingP2PoleArm",
];

pub const STAB_ACTIONS: &[&str] = &[
    "stabO1", "stabO2", "stabOF", "stabT1", "stabT2", "stabTF", "proneStab",
];

pub const MULTI_SWING_ACTIONS: &[&str] = &[
    "doubleSwing", "tripleSwing",
    "overSwingDouble", "overSwingTriple",
    "fullSwingDouble", "fullSwingTriple",
];

pub const RANGED_ACTIONS: &[&str] = &[
    "shoot1", "shoot2", "shoot6", "shootF",
    "shot", "handgun", "cannon", "torpedo",
];

pub const MAGIC_ACTIONS: &[&str] = &[
    "magic1", "magic2", "magic3", "magic4", "magic5",
    "magicmissile", "superMagicmissile",
    "heal", "holyShield", "genesis", "sanctuary",
    "resurrection", "timeleap", "meteor", "blizzard",
    "chainlightning", "firestrike", "fireburner", "flamegear",
    "icebreathe_prepare", "breathe_prepare",
    "shockwave", "windspear", "windshot", "stormbreak", "wave",
    "magicFlare", "magicBooster", "coolingeffect", "souldriver",
];

pub const MOVEMENT_SKILL_ACTIONS: &[&str] = &[
    "dash", "combatStep", "straight", "somersault", "rollingSpin",
    "backstep", "rush", "rush2",
    "ghostfly", "ghostjump", "ghostladder", "ghostproneStab",
    "ghostrope", "ghostsit", "ghoststand", "ghostwalk",
];

fn resolve_equipment(
    base: &crate::wz::Node,
    equipment: &[(EquipSlot, u32)],
) -> Vec<EquipmentEntry> {
    equipment
        .iter()
        .map(|(slot, item_id)| {
            let item_path = format!("Character/{}/{:08}.img", slot.dir_name(), item_id);
            let vslot = base
                .at_path(&item_path)
                .map(|n| load_vslot(&n))
                .unwrap_or_default();
            EquipmentEntry {
                slot: *slot,
                item_id: *item_id,
                vslot,
            }
        })
        .collect()
}

fn apply_vslot_filter(
    actions: HashMap<String, Vec<FrameData>>,
    face_expressions: HashMap<String, Vec<FrameData>>,
    equipment: &[EquipmentEntry],
) -> (
    HashMap<String, Vec<FrameData>>,
    HashMap<String, Vec<FrameData>>,
) {
    let filter_frame = |frame: FrameData| FrameData {
        parts: filter_hidden_sprites(frame.parts, equipment),
        delay: frame.delay,
        flip: frame.flip,
    };
    let actions = actions
        .into_iter()
        .map(|(name, frames)| (name, frames.into_iter().map(filter_frame).collect()))
        .collect();
    let face_expressions = face_expressions
        .into_iter()
        .map(|(name, frames)| (name, frames.into_iter().map(filter_frame).collect()))
        .collect();
    (actions, face_expressions)
}

fn build_part_entity(commands: &mut Commands, layer: &SpriteLayer, pos: Vec3) -> Entity {
    commands
        .spawn((
            Sprite::from_image(layer.image.clone()),
            Transform::from_translation(pos),
            CharacterPart {
                layer: layer.layer_name.clone(),
                z_base: layer.z,
            },
        ))
        .id()
}

fn update_part_sprite(
    part_entity: Entity,
    layer: &SpriteLayer,
    pos: Vec3,
    flip: bool,
    part_query: &mut Query<(&mut Sprite, &mut Transform, &CharacterPart)>,
) {
    if let Ok((mut sprite, mut transform, _)) = part_query.get_mut(part_entity) {
        sprite.image = layer.image.clone();
        transform.translation = if flip {
            // Mirror the origin (connection point) around root center on the x axis:
            //   origin_root = pos + origin
            //   new_origin_root.x = -origin_root.x
            //   new_pos.x + origin.x = -(pos.x + origin.x)
            //   new_pos.x = -pos.x - 2 * origin.x
            Vec3::new(
                -pos.x - 2.0 * layer.origin.x,
                pos.y,
                pos.z,
            )
        } else {
            pos
        };
        transform.scale = Vec3::ONE;
    }
}

pub fn spawn_character(
    trigger: On<SpawnCharacter>,
    mut commands: Commands,
    mut cache: ResMut<WzSpriteCache>,
    mut images: ResMut<Assets<Image>>,
    zmap: Res<ZMap>,
    slot_map: Res<SlotMap>,
) {
    let ev = trigger.event();
    info!("spawn_character at {:?}", ev.transform.translation);
    let base = get_cached_base();

    let loaded = loader::preload_character_frames(
        base,
        ev.config.skin_suffix,
        Some(ev.config.hair_id),
        Some(ev.config.face_id),
        &ev.config.equipment,
        &zmap,
        &slot_map,
        &mut cache,
        &mut images,
    );

    let equipment_entries = resolve_equipment(base, &ev.config.equipment);
    let (actions, face_expressions) =
        apply_vslot_filter(loaded.actions, loaded.face_expressions, &equipment_entries);

    let face_face_frames = face_expressions
        .get(&ev.face_expression)
        .cloned()
        .unwrap_or_default();
    let face_delay = face_face_frames.first().map(|f| f.delay).unwrap_or(2000);
    let first_frames = actions.get(&ev.action);
    let parts = first_frames
        .and_then(|frames| frames.first())
        .map(|f| &f.parts)
        .cloned()
        .unwrap_or_default();

    // Merge face expression parts into the initial frame
    let mut merged_parts = parts.clone();
    if let Some(face_frame) = face_face_frames.first() {
        merged_parts.extend(face_frame.parts.clone());
    }
    let positions = compute_frame_transforms(&merged_parts, GameLayer::Character.base_z());

    let delay = first_frames
        .and_then(|frames| frames.first())
        .map(|f| f.delay)
        .unwrap_or(100);
    let first_flip = first_frames
        .and_then(|frames| frames.first())
        .map(|f| f.flip)
        .unwrap_or(false);

    let pos = ev.transform.translation;
    let root = commands
        .spawn((
            CharacterRoot,
            ev.config.clone(),
            CharacterEquipment {
                entries: equipment_entries,
            },
            CharacterAnimation {
                action: ev.action.clone(),
                frame_idx: 0,
                timer: Timer::from_seconds(delay as f32 / 1000.0, TimerMode::Repeating),
                face_expression: ev.face_expression.clone(),
                face_frame_idx: 0,
                face_timer: Timer::from_seconds(face_delay as f32 / 1000.0, TimerMode::Repeating),
                flip: first_flip,
            },
            CharacterFrameData {
                actions,
                face_expressions,
            },
            ev.config.job,
            LearnedSkills::default(),
            PartEntities {
                map: HashMap::new(),
            },
            CharacterLayer(0),
            ev.transform,
            PhysicsState {
                x: pos.x,
                y: pos.y,
                vx: 0.0,
                vy: 0.0,
                on_fh: false,
                fh_id: 0,
                fh_group: 0,
                fh_layer: 0,
                left: false,
                right: false,
                up: false,
                down: false,
                jump_request: false,
                enable_gravity: true,
                enable_footholds: true,
            },
            CharacterIntent::default(),
            IsLocalPlayer,
        ))
        .id();

    let mut part_map = HashMap::new();
    let mut part_entities = Vec::new();

    for layer in &merged_parts {
        let pos = positions
            .get(&layer.layer_name)
            .copied()
            .unwrap_or(Vec3::ZERO);
        let child = build_part_entity(&mut commands, layer, pos);
        part_map.insert(layer.layer_name.clone(), child);
        part_entities.push(child);
    }

    commands.entity(root).add_children(&part_entities);
    commands.entity(root).insert(PartEntities { map: part_map });
}
    trigger: On<SetFlip>,
    mut query: Query<(&mut CharacterAnimation, &CharacterFrameData, &PartEntities)>,
    mut part_query: Query<(&mut Sprite, &mut Transform, &CharacterPart)>,
) {
    let ev = trigger.event();
    let Ok((mut anim, frame_data, part_entities)) = query.get_mut(ev.entity) else {
        return;
    };

    anim.flip = ev.facing_left;

    let Some(frames) = frame_data.actions.get(&anim.action) else {
        return;
    };
    let Some(frame) = frames.get(anim.frame_idx % frames.len().max(1)) else {
        return;
    };

    let mut merged_parts = frame.parts.clone();
    if let Some(face_frames) = frame_data.face_expressions.get(&anim.face_expression) {
        if let Some(face_frame) =
            face_frames.get(anim.face_frame_idx.min(face_frames.len().saturating_sub(1)))
        {
            merged_parts.extend(face_frame.parts.clone());
        }
    }

        let positions = compute_frame_transforms(&merged_parts, GameLayer::Character.base_z());

        for layer in &merged_parts {
            if let Some(&child) = part_entities.map.get(&layer.layer_name) {
                let pos = positions
                    .get(&layer.layer_name)
                    .copied()
                    .unwrap_or(Vec3::ZERO);
                update_part_sprite(child, layer, pos, anim.flip, &mut part_query);
            }
        }
    }
}

pub fn on_use_skill(
    trigger: On<UseSkill>,
    mut commands: Commands,
    skill_db: Res<SkillDatabase>,
    char_query: Query<&Transform, With<CharacterRoot>>,
) {
    let ev = trigger.event();
    let Some(skill) = skill_db.get(ev.skill_id) else {
        warn!("skill {} not found in database", ev.skill_id);
        return;
    };
    let Ok(char_transform) = char_query.get(ev.entity) else {
        return;
    };

    // Play character animation if the skill has one
    if let Some(action) = &skill.action {
        commands.trigger(SetAction {
            entity: ev.entity,
            action: action.clone(),
        });
    }

    // Spawn effect frames as children of character root
    if !skill.effect_frames.is_empty() {
        let effect_root = commands
            .spawn((
                SkillEffectRoot,
                Transform::from_translation(Vec3::ZERO),
                Visibility::Visible,
            ))
            .id();

        commands.entity(ev.entity).add_child(effect_root);

        let first_delay = skill.effect_frames.first().map(|f| f.delay).unwrap_or(100);
        commands.entity(effect_root).insert(SkillEffect {
            frames: skill.effect_frames.clone(),
            frame_idx: 0,
            timer: Timer::from_seconds(first_delay as f32 / 1000.0, TimerMode::Repeating),
            finished: false,
        });

        // Spawn first frame's sprite as child of effect root
        if let Some(first) = skill.effect_frames.first() {
            let sprite_entity = commands
                .spawn((
                    Sprite::from_image(first.image.clone()),
                    Transform::from_xyz(
                        -first.origin.x,
                        -first.origin.y,
                        first.z as f32,
                    ),
                    Visibility::Visible,
                ))
                .id();
            commands.entity(effect_root).add_child(sprite_entity);
        }
    }
}

pub fn animate_skill_effects(
    time: Res<Time>,
    mut effect_query: Query<(Entity, &mut SkillEffect, &Children), With<SkillEffectRoot>>,
    mut sprite_query: Query<(&mut Sprite, &mut Transform)>,
    mut commands: Commands,
) {
    for (entity, mut effect, children) in &mut effect_query {
        effect.timer.tick(time.delta());
        if !effect.timer.just_finished() {
            continue;
        }

        effect.frame_idx += 1;
        if effect.frame_idx >= effect.frames.len() {
            for &child in children.iter() {
                commands.entity(child).despawn();
            }
            commands.entity(entity).despawn();
            continue;
        }

        let frame = &effect.frames[effect.frame_idx];
        effect.timer =
            Timer::from_seconds(frame.delay as f32 / 1000.0, TimerMode::Repeating);

        if let Some(&child) = children.iter().next() {
            if let Ok((mut sprite, mut child_transform)) = sprite_query.get_mut(child) {
                sprite.image = frame.image.clone();
                child_transform.translation = Vec3::new(
                    -frame.origin.x,
                    -frame.origin.y,
                    frame.z as f32,
                );
            }
        }
    }
}

pub fn on_character_action(
    trigger: On<crate::input::ActionEvent>,
    query: Query<Entity, With<CharacterRoot>>,
    query_job: Query<&Job, With<CharacterRoot>>,
    mut cycle: ResMut<CharacterActionCycle>,
    skill_db: Res<SkillDatabase>,
    mut commands: Commands,
) {
    let entity = match query.iter().next() {
        Some(e) => e,
        None => return,
    };

    let anim = match trigger.event().0 {
        KeyAction::Stand1 => "stand1",
        KeyAction::Walk1 => "walk1",
        KeyAction::Jump | KeyAction::JumpAction => "jump",
        KeyAction::Sit => "sit",
        KeyAction::Prone => "prone",
        KeyAction::Ladder => "ladder",
        KeyAction::Rope => "rope",
        KeyAction::Fly => "fly",
        KeyAction::Alert => "alert",
        KeyAction::Dead => "dead",
        KeyAction::SwingO1 => "swingO1",
        KeyAction::SwingP1 => "swingP1",
        KeyAction::Shoot1 => "shoot1",
        KeyAction::Magic1 => "magic1",
        // Category cycling
        KeyAction::CycleStance => {
            let list = STANCE_ACTIONS;
            let a = list[cycle.stance % list.len()];
            cycle.stance = (cycle.stance + 1) % list.len();
            a
        }
        KeyAction::CycleAlert => {
            let list = ALERT_ACTIONS;
            let a = list[cycle.alert % list.len()];
            cycle.alert = (cycle.alert + 1) % list.len();
            a
        }
        KeyAction::CycleSwing => {
            let list = SWING_ACTIONS;
            let a = list[cycle.swing % list.len()];
            cycle.swing = (cycle.swing + 1) % list.len();
            a
        }
        KeyAction::CycleStab => {
            let list = STAB_ACTIONS;
            let a = list[cycle.stab % list.len()];
            cycle.stab = (cycle.stab + 1) % list.len();
            a
        }
        KeyAction::CycleMultiSwing => {
            let list = MULTI_SWING_ACTIONS;
            let a = list[cycle.multi_swing % list.len()];
            cycle.multi_swing = (cycle.multi_swing + 1) % list.len();
            a
        }
        KeyAction::CycleRanged => {
            let list = RANGED_ACTIONS;
            let a = list[cycle.ranged % list.len()];
            cycle.ranged = (cycle.ranged + 1) % list.len();
            a
        }
        KeyAction::CycleMagic => {
            let list = MAGIC_ACTIONS;
            let a = list[cycle.magic % list.len()];
            cycle.magic = (cycle.magic + 1) % list.len();
            a
        }
        KeyAction::CycleMovementSkill => {
            let list = MOVEMENT_SKILL_ACTIONS;
            let a = list[cycle.movement_skill % list.len()];
            cycle.movement_skill = (cycle.movement_skill + 1) % list.len();
            a
        }
        KeyAction::CycleSkill => {
            let skill_db = skill_db.as_ref();
            let job = match query_job.get(entity) {
                Ok(j) => j,
                Err(_) => return,
            };
            let active = skill_db.active_skills_for_job(&job.lineage());
            if active.is_empty() {
                return;
            }
            let idx = cycle.skill % active.len();
            cycle.skill = (cycle.skill + 1) % active.len();
            let skill_entry = active[idx];
            commands.trigger(UseSkill {
                entity,
                skill_id: skill_entry.id,
            });
            return;
        }
        KeyAction::FlipLeft => {
            commands.trigger(SetFlip { entity, facing_left: true });
            return;
        }
        KeyAction::FlipRight => {
            commands.trigger(SetFlip { entity, facing_left: false });
            return;
        }
        _ => return,
    };
    commands.trigger(SetAction {
        entity,
        action: anim.to_string(),
    });
}

pub fn animate_characters(
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &mut CharacterAnimation,
        &CharacterFrameData,
        &PartEntities,
    )>,
    mut part_query: Query<(&mut Sprite, &mut Transform, &CharacterPart)>,
) {
    for (entity, mut anim, frame_data, part_entities) in &mut query {
        let mut animation_dirty = false;

        anim.timer.tick(time.delta());
        if anim.timer.just_finished() {
            if let Some(frames) = frame_data.actions.get(&anim.action) {
                if !frames.is_empty() {
                    anim.frame_idx = (anim.frame_idx + 1) % frames.len();
                    let frame = &frames[anim.frame_idx];
                    anim.timer =
                        Timer::from_seconds(frame.delay as f32 / 1000.0, TimerMode::Repeating);
                    anim.flip = frame.flip;
                    animation_dirty = true;
                }
            }
        }

        anim.face_timer.tick(time.delta());
        if anim.face_timer.just_finished() {
            if let Some(face_frames) = frame_data.face_expressions.get(&anim.face_expression) {
                if !face_frames.is_empty() {
                    anim.face_frame_idx = (anim.face_frame_idx + 1) % face_frames.len();
                    let face_frame = &face_frames[anim.face_frame_idx];
                    let fd = face_frame.delay.max(50);
                    anim.face_timer = Timer::from_seconds(fd as f32 / 1000.0, TimerMode::Repeating);
                    animation_dirty = true;
                }
            }
        }

        if !animation_dirty {
            continue;
        }

        let Some(frames) = frame_data.actions.get(&anim.action) else {
            continue;
        };
        let frame = match frames.get(anim.frame_idx % frames.len().max(1)) {
            Some(f) => f,
            None => continue,
        };

        let mut merged_parts = frame.parts.clone();
        if let Some(face_frames) = frame_data.face_expressions.get(&anim.face_expression) {
            if let Some(face_frame) =
                face_frames.get(anim.face_frame_idx.min(face_frames.len().saturating_sub(1)))
            {
                merged_parts.extend(face_frame.parts.clone());
            }
        }

        let positions = compute_frame_transforms(&merged_parts, GameLayer::Character.base_z());

        for layer in &merged_parts {
            if let Some(&child) = part_entities.map.get(&layer.layer_name) {
                let pos = positions
                    .get(&layer.layer_name)
                    .copied()
                    .unwrap_or(Vec3::ZERO);
                update_part_sprite(child, layer, pos, anim.flip, &mut part_query);
            }
        }
    }
}

pub fn on_set_action(
    trigger: On<SetAction>,
    mut query: Query<(&mut CharacterAnimation, &CharacterFrameData, &PartEntities)>,
    mut part_query: Query<(&mut Sprite, &mut Transform, &CharacterPart)>,
) {
    let ev = trigger.event();
    let Ok((mut anim, frame_data, part_entities)) = query.get_mut(ev.entity) else {
        return;
    };

    let Some(frames) = frame_data.actions.get(&ev.action) else {
        warn!("action '{}' not found for character", ev.action);
        return;
    };

    anim.action = ev.action.clone();
    anim.frame_idx = 0;
    anim.flip = false;

    if let Some(first) = frames.first() {
        anim.timer = Timer::from_seconds(first.delay as f32 / 1000.0, TimerMode::Repeating);
        anim.flip = first.flip;

        let mut merged_parts = first.parts.clone();
        if let Some(face_frames) = frame_data.face_expressions.get(&anim.face_expression) {
            if let Some(face_frame) =
                face_frames.get(anim.face_frame_idx.min(face_frames.len().saturating_sub(1)))
            {
                merged_parts.extend(face_frame.parts.clone());
            }
        }

        let positions = compute_frame_transforms(&merged_parts, GameLayer::Character.base_z());

        for layer in &merged_parts {
            if let Some(&child) = part_entities.map.get(&layer.layer_name) {
                let pos = positions
                    .get(&layer.layer_name)
                    .copied()
                    .unwrap_or(Vec3::ZERO);
                update_part_sprite(child, layer, pos, anim.flip, &mut part_query);
            }
        }
    }
}
