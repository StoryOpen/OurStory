use bevy::prelude::*;
use bevy::sprite::Anchor;
use std::collections::{HashMap, HashSet};

use crate::character::components::*;
use crate::character::events::*;
use crate::character::skills::LearnedSkills;
use crate::character::types::*;
use crate::input::IsLocalPlayer;
use crate::physics::PhysicsState;

pub mod animation;
pub mod input;

pub use animation::*;
pub use input::*;

/// Shared constants used by spawn, animation, and input modules.
pub const DEFAULT_CHARACTER_ACTION: &str = "stand1";
pub(crate) const MIN_TIMER_SECS: f32 = 0.016;

const LABEL_Y: f32 = -48.0;
const LABEL_GAP: f32 = 4.0;

// ── Image loading ──

fn load_character_image(
    wz: &wz::WzData,
    path: &str,
    images: &mut Assets<Image>,
) -> (Handle<Image>, u32, u32) {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let dynamic_image = match wz.load_image(path) {
        Ok(img) => img,
        Err(e) => {
            warn!("failed to load character image at {path}: {e}");
            return (Handle::default(), 0, 0);
        }
    };
    let rgba = dynamic_image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let handle = images.add(Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba.into_raw(),
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ));
    (handle, width, height)
}

fn load_cached_part_image(
    wz: &wz::WzData,
    path: &str,
    cache: &mut HashMap<String, (Handle<Image>, u32, u32)>,
    images: &mut Assets<Image>,
) -> (Handle<Image>, u32, u32) {
    cache.get(path).cloned().unwrap_or_else(|| {
        let result = load_character_image(wz, path, images);
        cache.insert(path.to_string(), result.clone());
        result
    })
}

// ── Z resolution ──

fn resolve_z_frames(frames: &mut [wz::BodyFrame], zmap: &ZMap) {
    for frame in frames {
        for part in frame.parts.iter_mut() {
            if let Some(slot) = &part.slot {
                part.z = zmap.depth(slot);
            }
        }
    }
}

fn collect_raw_parts<'a>(
    body: &'a [wz::BodyFrame],
    hair: &'a [wz::BodyFrame],
    equips: &'a [Vec<wz::BodyFrame>],
    face: Option<&'a wz::BodyPart>,
    frame_idx: usize,
) -> HashMap<&'a str, &'a wz::BodyPart> {
    let mut raw_parts: HashMap<&str, &wz::BodyPart> = HashMap::new();
    for part in &body[frame_idx].parts {
        raw_parts.insert(&part.part_name, part);
    }
    let hair_frame = hair.get(frame_idx % hair.len().max(1));
    if let Some(hf) = hair_frame {
        for part in &hf.parts {
            raw_parts.insert(&part.part_name, part);
        }
    }
    for eq in equips {
        if let Some(f) = eq.get(frame_idx % eq.len().max(1)) {
            for part in &f.parts {
                raw_parts.insert(&part.part_name, part);
            }
        }
    }
    if let Some(face_part) = face {
        raw_parts.insert("face", face_part);
    }
    raw_parts
}

fn compute_body_part_positions<'a>(
    raw_parts: &'a HashMap<&'a str, &'a wz::BodyPart>,
) -> HashMap<&'a str, Vec3> {
    const PART_ORDER: &[&str] = &["body", "head", "arm"];

    let mut sorted: Vec<(&str, &wz::BodyPart)> =
        raw_parts.iter().map(|(&k, &v)| (k, v)).collect();
    sorted.sort_by_key(|(name, _)| {
        PART_ORDER
            .iter()
            .position(|&o| o == *name)
            .unwrap_or(PART_ORDER.len())
    });

    let mut connection_points: HashMap<&str, Vec2> = HashMap::new();
    let mut transforms: HashMap<&str, Vec3> = HashMap::new();

    for (part_name, part) in &sorted {
        if *part_name == "body" {
            transforms.insert(part_name, Vec3::new(0.0, 0.0, part.z));
            for (key, v) in &part.map {
                connection_points.insert(key.as_str(), Vec2::new(v.0, v.1));
            }
            continue;
        }

        let matched = connection_points
            .keys()
            .find(|k| part.map.contains_key(**k))
            .copied();

        let Some(conn_key) = matched else {
            continue;
        };

        let cp = connection_points[conn_key];
        let me = part.map[conn_key];
        let tx = cp.x - me.0;
        let ty = cp.y - me.1;

        transforms.insert(part_name, Vec3::new(tx, ty, part.z));

        for (key, v) in &part.map {
            if key != conn_key {
                let cp_new = Vec2::new(tx + v.0, ty + v.1);
                connection_points.insert(key.as_str(), cp_new);
            }
        }
    }

    transforms
}

// ── Frame loading helpers ──

fn load_body_frames(wz: &wz::WzData, skin: u32, action: &str, zmap: &ZMap) -> Vec<wz::BodyFrame> {
    let mut frames = wz
        .load_character_body(skin, action)
        .unwrap_or_else(|e| panic!("action '{}' failed to load body frames: {:?}", action, e))
        .frames
        .clone();
    assert!(!frames.is_empty(), "action '{}' has no body frames", action);
    resolve_z_frames(&mut frames, zmap);
    frames
}

fn load_hair_frames(
    wz: &wz::WzData,
    hair_id: u32,
    action: &str,
    zmap: &ZMap,
) -> Vec<wz::BodyFrame> {
    let mut frames = wz
        .load_hair_body(hair_id, action)
        .map(|hair| hair.frames.clone())
        .unwrap_or_default();
    resolve_z_frames(&mut frames, zmap);
    frames
}

fn load_equip_frames(
    wz: &wz::WzData,
    equipment: &[(crate::character::types::EquipSlot, u32)],
    action: &str,
    zmap: &ZMap,
) -> Vec<Vec<wz::BodyFrame>> {
    equipment
        .iter()
        .filter_map(|(_slot, item_id)| {
            let eq_action = wz.load_equip_action(*item_id as i32, action).ok()?;
            if eq_action.frames.is_empty() {
                return None;
            }
            let mut frames = eq_action.frames.clone();
            resolve_z_frames(&mut frames, zmap);
            Some(frames)
        })
        .collect()
}

// ── Pose baking ──

#[derive(Clone)]
pub struct ActionFrame {
    pub delay_ms: u32,
    pub parts: HashMap<String, PartPose>,
}

fn load_action_frames(
    wz: &wz::WzData,
    images: &mut Assets<Image>,
    zmap: &ZMap,
    config: &CharacterConfig,
    action: &str,
    face_expression: &str,
) -> Vec<ActionFrame> {
    let body = load_body_frames(wz, config.skin_suffix, action, zmap);
    let hair = load_hair_frames(wz, config.hair_id, action, zmap);
    let equips = load_equip_frames(wz, &config.equipment, action, zmap);
    let face = wz
        .load_face_expression(config.face_id, face_expression)
        .ok()
        .and_then(|fe| {
            fe.frames.first().map(|f| {
                let mut part = f.part.clone();
                if let Some(slot) = &part.slot {
                    part.z = zmap.depth(slot);
                }
                part
            })
        });

    let frame_count = body.len();

    let mut img_cache: HashMap<String, (Handle<Image>, u32, u32)> = HashMap::new();
    let mut frames = Vec::with_capacity(frame_count);

    for frame_idx in 0..frame_count {
        let raw_parts = collect_raw_parts(&body, &hair, &equips, face.as_ref(), frame_idx);
        let transforms = compute_body_part_positions(&raw_parts);

        let mut parts = HashMap::new();
        for (&part_name, part) in &raw_parts {
            let pos = *transforms.get(part_name).unwrap_or(&Vec3::ZERO);
            let (handle, w, h) =
                load_cached_part_image(wz, &part.image_path, &mut img_cache, images);
            let anchor = compute_anchor(part.origin, w as f32, h as f32);
            parts.insert(
                part_name.to_string(),
                PartPose {
                    image: handle,
                    position: pos,
                    anchor,
                    visible: true,
                },
            );
        }

        let delay_ms = body[frame_idx].delay;

        frames.push(ActionFrame {
            delay_ms,
            parts,
        });
    }

    frames
}

fn compute_anchor(origin: wz::Vector2D, img_width: f32, img_height: f32) -> Vec2 {
    if img_width == 0.0 || img_height == 0.0 {
        return Vec2::ZERO;
    }
    Vec2::new(
        origin.0 / img_width - 0.5,
        origin.1 / img_height - 0.5,
    )
}

// ── Labels ──

pub fn draw_character_labels(
    query: Query<(&Transform, &CharacterLabels), With<CharacterRoot>>,
    mut gizmos: Gizmos,
) {
    for (transform, labels) in &query {
        let pos = transform.translation.truncate();
        let action_pos = pos + Vec2::new(-LABEL_GAP, LABEL_Y);
        let job_pos = pos + Vec2::new(LABEL_GAP, LABEL_Y);
        gizmos.text_2d(
            Isometry2d::from_translation(action_pos),
            &labels.action,
            12.0,
            Vec2::new(1.0, 0.5),
            Color::WHITE,
        );
        gizmos.text_2d(
            Isometry2d::from_translation(job_pos),
            &labels.job,
            12.0,
            Vec2::new(-1.0, 0.5),
            Color::srgb(1.0, 0.88, 0.45),
        );
    }
}

// ── Entity construction ──

fn transform_action_to_entities(
    commands: &mut Commands,
    frames: &[ActionFrame],
    character_root: Entity,
    character_body: Option<Entity>,
    body_children: &Children,
    part_query: &Query<&PartName>,
) -> Option<Entity> {
    commands
        .entity(character_root)
        .insert(CurrentAction {
            frames: frames.to_vec(),
        });

    let mut part_names: Vec<String> = frames
        .iter()
        .flat_map(|f| f.parts.keys().cloned())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    part_names.sort_by(|a, b| {
        if a == "body" {
            std::cmp::Ordering::Less
        } else if b == "body" {
            std::cmp::Ordering::Greater
        } else {
            a.cmp(b)
        }
    });

    let mut existing_body_entities: HashMap<String, Entity> = body_children
        .iter()
        .filter_map(|child| {
            part_query
                .get(child)
                .ok()
                .map(|pn| (pn.0.clone(), child))
        })
        .collect();

    if let Some(body_entity) = character_body {
        existing_body_entities.insert("body".to_string(), body_entity);
    };

    let first_frame = &frames[0];
    let mut current_body_entity = character_body;
    let mut face_entity = None;

    info!(
        "[transform_action] {} parts to spawn: {:?}",
        part_names.len(),
        part_names
    );

    for name in &part_names {
        let pose = first_frame
            .parts
            .get(name)
            .cloned()
            .unwrap_or_else(PartPose::hidden);

        let entity = if let Some(&e) = existing_body_entities.get(name) {
            e
        } else {
            let e = commands
                .spawn((
                    Name::new(format!("Part:{name}")),
                    PartName(name.clone()),
                    Sprite::default(),
                    Transform::default(),
                    Visibility::Hidden,
                ))
                .id();
            if name == "body" {
                commands.entity(e).insert(CharacterBody);
                commands.entity(character_root).add_child(e);
                current_body_entity = Some(e);
            } else {
                let body_entity = current_body_entity.expect("body must be spawned first");
                commands.entity(body_entity).add_child(e);
            }
            e
        };
        if name == "face" {
            face_entity = Some(entity);
        }
        commands.entity(entity).insert((
            Sprite::from_image(pose.image),
            Transform::from_translation(pose.position),
            Visibility::Hidden,
            Anchor(pose.anchor),
        ));
    }

    face_entity
}

pub fn spawn_character(
    trigger: On<SpawnCharacter>,
    mut commands: Commands,
    job_catalog: Res<crate::character::job::JobCatalog>,
) {
    let ev = trigger.event();
    let pos = ev.transform.translation;

    let root = commands
        .spawn((
            Name::new("Character"),
            CharacterRoot,
            ev.config.clone(),
            CharacterActionAnimation {
                action: ev.action.clone(),
                default_action: DEFAULT_CHARACTER_ACTION.to_string(),
                return_to_default: false,
                pending_action: None,
                frame_idx: 0,
                timer: Timer::from_seconds(0.1, TimerMode::Repeating),
                facing_left: true,
                frame_count: 0,
            },
            CharacterFaceAnimation {
                expression: ev.face_expression.clone(),
                frame_idx: 0,
                timer: Timer::from_seconds(1.0, TimerMode::Repeating),
                frames: Vec::new(),
                face_entity: None,
            },
            crate::character::stance::CharacterStance::default(),
            ev.config.job,
            LearnedSkills::default(),
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
        ))
        .insert(IsLocalPlayer)
        .id();

    let job_label_text = job_catalog.display_label(ev.config.job);
    commands.entity(root).insert(CharacterLabels {
        action: ev.action.clone(),
        job: job_label_text,
    });

    commands.trigger(SetAction {
        entity: root,
        action: ev.action.clone(),
        return_to_default: false,
    });

    commands.trigger(SetFaceExpression {
        entity: root,
        expression: ev.face_expression.clone(),
        action: ev.action.clone(),
    });
}

// ── Action loading ──

pub fn on_set_action(
    trigger: On<SetAction>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    wz: Res<WzDataRes>,
    zmap: Res<ZMap>,
    mut query: Query<
        (
            Entity,
            &mut CharacterActionAnimation,
            &CharacterConfig,
            Option<&Children>,
            &mut CharacterLabels,
            &mut CharacterFaceAnimation,
        ),
        With<CharacterRoot>,
    >,
    body_query: Query<&Children, With<CharacterBody>>,
    part_query: Query<&PartName>,
) {
    let ev = trigger.event();
    let root = ev.entity;
    let Ok((character_root, mut anim, config, children, mut labels, mut face_anim)) =
        query.get_mut(root)
    else {
        return;
    };

    let default_children = Children::default();
    let children_ref = children.unwrap_or(&default_children);
    let (body_entity, body_children) = match children_ref
        .iter()
        .find_map(|child| body_query.get(child).ok().map(|c| (child, c)))
    {
        Some((entity, children)) => (Some(entity), children),
        None => (None, &default_children),
    };

    if anim.return_to_default {
        anim.pending_action = Some(PendingCharacterAction::Action {
            action: ev.action.clone(),
            return_to_default: ev.return_to_default,
        });
        return;
    }

    let action_frames = load_action_frames(
        &wz,
        &mut images,
        &zmap,
        config,
        &ev.action,
        &face_anim.expression,
    );

    let returned_face = transform_action_to_entities(
        &mut commands,
        &action_frames,
        character_root,
        body_entity,
        body_children,
        &part_query,
    );

    if face_anim.face_entity.is_none() {
        face_anim.face_entity = returned_face;
    }

    anim.action = ev.action.clone();
    anim.return_to_default = ev.return_to_default;
    anim.frame_idx = 0;
    anim.frame_count = action_frames.len();
    if let Some(first) = action_frames.first() {
        let secs = (first.delay_ms as f32 / 1000.0).max(MIN_TIMER_SECS);
        anim.timer = Timer::from_seconds(secs, TimerMode::Repeating);
    }
    info!(
        "[char action] loaded '{}' | {} frames | parts_per_frame: {:?}",
        ev.action,
        action_frames.len(),
        action_frames
            .first()
            .map(|f| f.parts.keys().collect::<Vec<_>>()),
    );

    labels.action.clone_from(&ev.action);
}

fn load_face_expression_image_frames(
    wz: &wz::WzData,
    images: &mut Assets<Image>,
    face_id: u32,
    expression: &str,
) -> Vec<FaceFrame> {
    let Ok(face_expr) = wz.load_face_expression(face_id, expression) else {
        return Vec::new();
    };
    let mut img_cache: HashMap<String, (Handle<Image>, u32, u32)> = HashMap::new();
    face_expr
        .frames
        .iter()
        .map(|f| {
            let (handle, w, h) =
                load_cached_part_image(wz, &f.part.image_path, &mut img_cache, images);
            FaceFrame {
                image: handle,
                anchor: compute_anchor(f.part.origin, w as f32, h as f32),
                delay_ms: f.delay,
            }
        })
        .collect()
}

pub fn on_set_face_expression(
    trigger: On<SetFaceExpression>,
    mut query: Query<
        (&CharacterConfig, &mut CharacterFaceAnimation, Option<&Children>),
        With<CharacterRoot>,
    >,
    mut part_query: Query<(
        &PartName,
        &mut Sprite,
        &mut Visibility,
        &mut Anchor,
    )>,
    wz: Res<WzDataRes>,
    mut images: ResMut<Assets<Image>>,
) {
    let ev = trigger.event();
    let Ok((config, mut face_anim, _root_children)) = query.get_mut(ev.entity) else {
        warn!("[on_set_face_expression] failed to query entity");
        return;
    };

    face_anim.expression = ev.expression.clone();
    face_anim.frame_idx = 0;
    face_anim.frames = load_face_expression_image_frames(
        &wz,
        &mut images,
        config.face_id,
        &ev.expression,
    );

    let first = face_anim.frames.first().cloned();
    let face_entity = face_anim.face_entity;
    if let Some(first) = first {
        let delay_secs = (first.delay_ms as f32 / 1000.0).max(MIN_TIMER_SECS);
        face_anim.timer = Timer::from_seconds(delay_secs, TimerMode::Repeating);

        if let Some(fe) = face_entity {
            match part_query.get_mut(fe) {
                Ok((_pn, mut sprite, mut visibility, mut anchor)) => {
                    sprite.image = first.image;
                    *anchor = Anchor(first.anchor);
                    *visibility = Visibility::Visible;
                }
                Err(e) => {
                    warn!(
                        "[on_set_face_expression] FAILED to get face entity {:?}: {:?}",
                        fe, e
                    );
                }
            }
        }
    }
}
