use std::collections::HashMap;
use bevy::prelude::*;

use crate::character::components::*;
use crate::character::events::*;
use crate::character::loader::{self, WzSpriteCache};
use crate::character::types::*;
use crate::input::{CharacterIntent, IsLocalPlayer};
use crate::physics::PhysicsState;
use crate::wz::get_cached_base;

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
) -> (HashMap<String, Vec<FrameData>>, HashMap<String, Vec<FrameData>>) {
    let filter_frame = |frame: FrameData| FrameData {
        parts: filter_hidden_sprites(frame.parts, equipment),
        delay: frame.delay,
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

fn build_part_entity(
    commands: &mut Commands,
    layer: &SpriteLayer,
    pos: Vec3,
) -> Entity {
    commands.spawn((
        Sprite::from_image(layer.image.clone()),
        Transform::from_translation(pos),
        CharacterPart {
            layer: layer.layer_name.clone(),
            z_base: layer.z,
        },
    )).id()
}

fn update_part_sprite(
    part_entity: Entity,
    layer: &SpriteLayer,
    pos: Vec3,
    part_query: &mut Query<(&mut Sprite, &mut Transform, &CharacterPart)>,
) {
    if let Ok((mut sprite, mut transform, _)) = part_query.get_mut(part_entity) {
        sprite.image = layer.image.clone();
        transform.translation = pos;
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
    let (actions, face_expressions) = apply_vslot_filter(
        loaded.actions,
        loaded.face_expressions,
        &equipment_entries,
    );

    let face_face_frames = face_expressions.get(&ev.face_expression).cloned().unwrap_or_default();
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
    let positions = compute_frame_transforms(&merged_parts);

    let delay = first_frames
        .and_then(|frames| frames.first())
        .map(|f| f.delay)
        .unwrap_or(100);

    let pos = ev.transform.translation;
    let root = commands.spawn((
        CharacterRoot,
        ev.config.clone(),
        CharacterEquipment { entries: equipment_entries },
        CharacterAnimation {
            action: ev.action.clone(),
            frame_idx: 0,
            timer: Timer::from_seconds(delay as f32 / 1000.0, TimerMode::Repeating),
            face_expression: ev.face_expression.clone(),
            face_frame_idx: 0,
            face_timer: Timer::from_seconds(face_delay as f32 / 1000.0, TimerMode::Repeating),
        },
        CharacterFrameData { actions, face_expressions },
        PartEntities { map: HashMap::new() },
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
    )).id();

    let mut part_map = HashMap::new();
    let mut part_entities = Vec::new();

    for layer in &merged_parts {
        let pos = positions.get(&layer.layer_name).copied().unwrap_or(Vec3::ZERO);
        let child = build_part_entity(&mut commands, layer, pos);
        part_map.insert(layer.layer_name.clone(), child);
        part_entities.push(child);
    }

    commands.entity(root).add_children(&part_entities);
    commands.entity(root).insert(PartEntities { map: part_map });
}

pub fn on_character_action(
    trigger: On<crate::input::ActionEvent>,
    query: Query<Entity, With<CharacterRoot>>,
    mut commands: Commands,
) {
    use crate::input::KeyAction;
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
        _ => return,
    };
    commands.trigger(SetAction { entity, action: anim.to_string() });
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
                    anim.timer = Timer::from_seconds(frame.delay as f32 / 1000.0, TimerMode::Repeating);
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
            if let Some(face_frame) = face_frames.get(anim.face_frame_idx.min(face_frames.len().saturating_sub(1))) {
                merged_parts.extend(face_frame.parts.clone());
            }
        }

        let positions = compute_frame_transforms(&merged_parts);

        for layer in &merged_parts {
            if let Some(&child) = part_entities.map.get(&layer.layer_name) {
                let pos = positions.get(&layer.layer_name).copied().unwrap_or(Vec3::ZERO);
                update_part_sprite(child, layer, pos, &mut part_query);
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

    if let Some(first) = frames.first() {
        anim.timer = Timer::from_seconds(first.delay as f32 / 1000.0, TimerMode::Repeating);

        let mut merged_parts = first.parts.clone();
        if let Some(face_frames) = frame_data.face_expressions.get(&anim.face_expression) {
            if let Some(face_frame) = face_frames.get(anim.face_frame_idx.min(face_frames.len().saturating_sub(1))) {
                merged_parts.extend(face_frame.parts.clone());
            }
        }

        let positions = compute_frame_transforms(&merged_parts);

        for layer in &merged_parts {
            if let Some(&child) = part_entities.map.get(&layer.layer_name) {
                let pos = positions.get(&layer.layer_name).copied().unwrap_or(Vec3::ZERO);
                update_part_sprite(child, layer, pos, &mut part_query);
            }
        }
    }
}
