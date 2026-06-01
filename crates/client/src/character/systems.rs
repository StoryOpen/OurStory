use std::collections::HashMap;
use bevy::{prelude::*, sprite::Anchor};

use crate::character::components::*;
use crate::character::events::*;
use crate::character::loader::{self, WzSpriteCache};
use crate::character::types::*;
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
        Anchor::TOP_LEFT,
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
        ev.transform,
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

pub fn character_action_controls(
    input: Res<ButtonInput<KeyCode>>,
    query: Query<Entity, With<CharacterRoot>>,
    mut commands: Commands,
) {
    let entity = match query.iter().next() {
        Some(e) => e,
        None => return,
    };

    let action = if input.just_pressed(KeyCode::Digit1) { "stand1" }
    else if input.just_pressed(KeyCode::Digit2) { "walk1" }
    else if input.just_pressed(KeyCode::Digit3) { "jump" }
    else if input.just_pressed(KeyCode::Digit4) { "sit" }
    else if input.just_pressed(KeyCode::Digit5) { "prone" }
    else if input.just_pressed(KeyCode::Digit6) { "ladder" }
    else if input.just_pressed(KeyCode::Digit7) { "rope" }
    else if input.just_pressed(KeyCode::Digit8) { "fly" }
    else if input.just_pressed(KeyCode::Digit9) { "alert" }
    else if input.just_pressed(KeyCode::Digit0) { "dead" }
    else if input.just_pressed(KeyCode::KeyQ) { "swingO1" }
    else if input.just_pressed(KeyCode::KeyW) { "swingP1" }
    else if input.just_pressed(KeyCode::KeyE) { "shoot1" }
    else if input.just_pressed(KeyCode::KeyR) { "magic1" }
    else { return };

    commands.trigger(SetAction { entity, action: action.to_string() });
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
