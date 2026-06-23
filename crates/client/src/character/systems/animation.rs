use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::character::components::*;
use crate::character::events::*;
use crate::character::skills::{SkillEffect, SkillEffectRoot};
use crate::input::IsLocalPlayer;
use crate::layer::GameLayer;
use crate::physics::PhysicsState;

use super::{DEFAULT_CHARACTER_ACTION, MIN_TIMER_SECS};

// ── Utilities ──

fn zig_zag(frame: usize, count: usize) -> usize {
    if count <= 1 {
        return 0;
    }
    let period = 2 * (count - 1);
    let pos = frame % period;
    if pos < count {
        pos
    } else {
        period - pos
    }
}

// ── Body animation tick ──

fn tick_body_animation(
    time: &Time,
    anim: &mut CharacterActionAnimation,
    action: &CurrentAction,
    part_entities: &[Entity],
    part_query: &mut Query<(
        &PartName,
        &mut Sprite,
        &mut Transform,
        &mut Visibility,
        &mut Anchor,
    )>,
    commands: &mut Commands,
    entity: Entity,
) {
    anim.timer.tick(time.delta());
    if anim.timer.just_finished() {
        let total_frames = if anim.frame_count > 1 {
            2 * (anim.frame_count - 1)
        } else {
            1
        };
        let at_end = anim.frame_idx + 1 >= total_frames;
        if anim.return_to_default && at_end {
            let pending_action = anim.pending_action.take();
            let default_action = anim.default_action.clone();
            anim.return_to_default = false;
            match pending_action {
                Some(PendingCharacterAction::Action {
                    action,
                    return_to_default,
                }) => {
                    commands.trigger(SetAction {
                        entity,
                        action,
                        return_to_default,
                    });
                }
                Some(PendingCharacterAction::Skill { skill_id }) => {
                    commands.trigger(UseSkill { entity, skill_id });
                }
                None => {
                    commands.trigger(SetAction {
                        entity,
                        action: default_action,
                        return_to_default: false,
                    });
                }
            }
            return;
        }

        anim.frame_idx += 1;
    }

    let display_frame = if anim.frame_count > 0 {
        zig_zag(anim.frame_idx, anim.frame_count)
    } else {
        0
    };
    let frame = &action.frames[display_frame];

    for &child in part_entities {
        let Ok((part_name, mut sprite, mut transform, mut visibility, mut anchor)) =
            part_query.get_mut(child)
        else {
            continue;
        };

        if let Some(pose) = frame.parts.get(part_name.0.as_str()) {
            *visibility = if pose.visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            sprite.image = pose.image.clone();
            transform.translation =
                Vec3::new(pose.position.x, pose.position.y, pose.position.z);
            *anchor = Anchor(pose.anchor);
        } else {
            *visibility = Visibility::Hidden;
        }
    }

    // Apply facing to the first entity (the body root)
    if let Some(&body_entity) = part_entities.first() {
        if let Ok((_, _, mut transform, _, _)) = part_query.get_mut(body_entity) {
            transform.scale.x = if anim.facing_left { 1.0 } else { -1.0 };
        }
    }
}

// ── Body animation system ──

pub fn advance_character_frames(
    time: Res<Time>,
    mut query: Query<
        (
            Entity,
            &mut CharacterActionAnimation,
            &Children,
            &CurrentAction,
        ),
        With<CharacterRoot>,
    >,
    body_query: Query<&Children, With<CharacterBody>>,
    mut part_query: Query<(
        &PartName,
        &mut Sprite,
        &mut Transform,
        &mut Visibility,
        &mut Anchor,
    )>,
    mut commands: Commands,
) {
    for (entity, mut anim, children, action) in &mut query {
        if anim.frame_count == 0 {
            continue;
        }

        let mut part_entities: Vec<Entity> = Vec::new();
        if let Some((body_ent, body_kids)) = children.iter().find_map(|child| {
            body_query.get(child).ok().map(|c| (child, c))
        }) {
            part_entities.push(body_ent);
            part_entities.extend(body_kids.iter());
        } else {
            part_entities.extend(children.iter());
        }

        tick_body_animation(
            &time,
            &mut anim,
            action,
            &part_entities,
            &mut part_query,
            &mut commands,
            entity,
        );
    }
}

// ── Face expression tick ──

fn tick_face_expression(
    time: &Time,
    face_state: &mut CharacterFaceAnimation,
) -> Option<FaceFrame> {
    if face_state.frames.is_empty() {
        return None;
    }
    face_state.timer.tick(time.delta());
    if !face_state.timer.just_finished() {
        return None;
    }

    let next_idx = (face_state.frame_idx + 1) % face_state.frames.len().max(1);
    face_state.frame_idx = next_idx;

    let frame = face_state.frames.get(next_idx).cloned();

    let delay_secs = if next_idx == 0 {
        1.0
    } else {
        face_state
            .frames
            .get(next_idx)
            .map(|f| f.delay_ms as f32 / 1000.0)
            .unwrap_or(0.1)
    };
    face_state.timer =
        Timer::from_seconds(delay_secs.max(MIN_TIMER_SECS), TimerMode::Repeating);

    frame
}

// ── Face animation system ──

pub fn animate_face(
    time: Res<Time>,
    mut query: Query<(&mut CharacterFaceAnimation, &Children), With<CharacterRoot>>,
    mut part_query: Query<(
        &PartName,
        &mut Sprite,
        &mut Visibility,
        &mut Anchor,
    )>,
) {
    for (mut face_state, _children) in &mut query {
        let face_entity = face_state.face_entity;

        if let Some(face_frame) = tick_face_expression(&time, &mut face_state) {
            if let Some(fe) = face_entity {
                if let Ok((_, mut sprite, mut visibility, mut anchor)) = part_query.get_mut(fe) {
                    sprite.image = face_frame.image;
                    *anchor = Anchor(face_frame.anchor);
                    *visibility = Visibility::Visible;
                }
            }
        }
    }
}

// ── Facing ──

pub fn on_set_facing(
    trigger: On<SetFacing>,
    mut anim_query: Query<&mut CharacterActionAnimation>,
) {
    let ev = trigger.event();
    let Ok(mut anim) = anim_query.get_mut(ev.entity) else {
        return;
    };
    anim.facing_left = ev.facing_left;
}

pub fn update_character_facing_from_intent(
    mut commands: Commands,
    mut query: Query<
        (&PhysicsState, &mut CharacterActionAnimation, Entity),
        (With<CharacterRoot>, With<IsLocalPlayer>),
    >,
) {
    for (phys, anim, entity) in &mut query {
        let moving = phys.left || phys.right;
        let walking = moving && anim.action == "stand1";
        let idle = !moving && anim.action == "walk1";

        if walking {
            commands.trigger(SetAction {
                entity,
                action: "walk1".to_string(),
                return_to_default: false,
            });
        } else if idle {
            commands.trigger(SetAction {
                entity,
                action: DEFAULT_CHARACTER_ACTION.to_string(),
                return_to_default: false,
            });
        }

        if !moving {
            continue;
        }
        let facing_left = phys.left;
        if anim.facing_left == facing_left {
            continue;
        }
        commands.trigger(SetFacing {
            entity,
            facing_left,
        });
    }
}

// ── Skills ──

pub fn on_use_skill(
    trigger: On<UseSkill>,
    mut commands: Commands,
    skill_db: Res<crate::character::skills::SkillDatabase>,
    mut char_query: Query<&mut CharacterActionAnimation, With<CharacterRoot>>,
) {
    let ev = trigger.event();
    let target = ev.entity;
    let Some(skill) = skill_db.get(ev.skill_id) else {
        warn!("skill {} not found in database", ev.skill_id);
        return;
    };
    let Ok(mut anim) = char_query.get_mut(target) else {
        return;
    };
    if anim.return_to_default {
        anim.pending_action = Some(PendingCharacterAction::Skill {
            skill_id: ev.skill_id,
        });
        return;
    }

    if skill.effect_frames.is_empty() {
        return;
    }

    if let Some(action) = &skill.action {
        commands.trigger(SetAction {
            entity: target,
            action: action.clone(),
            return_to_default: true,
        });
    }

    let effect_root = commands
        .spawn((
            Name::new("SkillEffect"),
            SkillEffectRoot,
            Transform::from_translation(Vec3::new(
                0.0,
                0.0,
                GameLayer::Skill.with_offset(0.0),
            )),
            Visibility::Visible,
        ))
        .id();

    commands.entity(target).add_child(effect_root);

    let first_delay = skill.effect_frames.first().map(|f| f.delay).unwrap_or(100);
    commands.entity(effect_root).insert(SkillEffect {
        frames: skill.effect_frames.clone(),
        frame_idx: 0,
        timer: Timer::from_seconds(
            (first_delay as f32 / 1000.0).max(MIN_TIMER_SECS),
            TimerMode::Repeating,
        ),
        finished: false,
        label: Some(skill.name.clone()),
    });

    if let Some(first) = skill.effect_frames.first() {
        let sprite_entity = commands
            .spawn((
                Sprite::from_image(first.image.clone()),
                Transform::from_xyz(-first.origin.x, -first.origin.y, first.z as f32),
                Visibility::Visible,
            ))
            .id();
        commands.entity(effect_root).add_child(sprite_entity);
    }
}

pub fn animate_skill_effects(
    time: Res<Time>,
    mut effect_query: Query<(Entity, &mut SkillEffect, &GlobalTransform, &Children), With<SkillEffectRoot>>,
    mut sprite_query: Query<(&mut Sprite, &mut Transform)>,
    mut commands: Commands,
    mut gizmos: Gizmos,
) {
    for (entity, mut effect, global_transform, children) in &mut effect_query {
        effect.timer.tick(time.delta());
        if !effect.timer.just_finished() {
            continue;
        }

        effect.frame_idx += 1;
        if effect.frame_idx >= effect.frames.len() {
            commands.entity(entity).despawn_children();
            commands.entity(entity).despawn();
            continue;
        }

        let frame = &effect.frames[effect.frame_idx];
        let image = frame.image.clone();
        let origin = frame.origin;
        let z = frame.z;
        let delay = frame.delay;
        effect.timer = Timer::from_seconds(
            (delay as f32 / 1000.0).max(MIN_TIMER_SECS),
            TimerMode::Repeating,
        );

        if let Some(child) = children.iter().next() {
            if let Ok((mut sprite, mut child_transform)) = sprite_query.get_mut(child) {
                sprite.image = image;
                child_transform.translation = Vec3::new(-origin.x, -origin.y, z as f32);
            }
        }

        if let Some(ref label_text) = effect.label {
            let label_pos = global_transform.translation().truncate() + Vec2::new(0.0, -60.0);
            gizmos.text_2d(
                Isometry2d::from_translation(label_pos),
                label_text,
                12.0,
                Vec2::ZERO,
                Color::WHITE,
            );
        }
    }
}
