use bevy::prelude::*;
use bevy::sprite::Anchor;
use std::collections::{HashMap, HashSet};

use crate::character::components::*;
use crate::character::events::*;
use crate::character::skills::LearnedSkills;
use crate::character::types::*;
use crate::input::IsLocalPlayer;
use crate::physics::PhysicsState;
use crate::wz::asset_loaders::*;

pub mod animation;
pub mod input;

pub use animation::*;
pub use input::*;

/// Shared constants used by spawn, animation, and input modules.
pub const DEFAULT_CHARACTER_ACTION: &str = "stand1";
pub(crate) const MIN_TIMER_SECS: f32 = 0.016;

const LABEL_Y: f32 = -48.0;
const LABEL_GAP: f32 = 4.0;

// ── Image loading via asset system ──

fn load_part_image(asset_server: &AssetServer, path: &str) -> Handle<Image> {
    asset_server.load::<Image>(format!("wz://{path}.wzimg"))
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

// ── Pose baking (combines all component frames into ActionFrames) ──

#[derive(Clone)]
pub struct ActionFrame {
    pub delay_ms: u32,
    pub parts: HashMap<String, PartPose>,
}

/// Combine body, hair, equip, and face data into ActionFrames with image handles.
fn combine_action_frames(
    asset_server: &AssetServer,
    body: &[wz::BodyFrame],
    hair: &[wz::BodyFrame],
    equips: &[Vec<wz::BodyFrame>],
    face: Option<&wz::BodyPart>,
) -> Vec<ActionFrame> {
    let frame_count = body.len();
    let mut frames = Vec::with_capacity(frame_count);

    for frame_idx in 0..frame_count {
        let raw_parts = collect_raw_parts(body, hair, equips, face, frame_idx);
        let transforms = compute_body_part_positions(&raw_parts);

        let mut parts = HashMap::new();
        for (&part_name, part) in &raw_parts {
            let pos = *transforms.get(part_name).unwrap_or(&Vec3::ZERO);
            let handle = load_part_image(asset_server, &part.image_path);
            parts.insert(
                part_name.to_string(),
                PartPose {
                    image: handle,
                    position: pos,
                    anchor: Vec2::new(-0.5, -0.5),
                    visible: true,
                },
            );
        }

        frames.push(ActionFrame {
            delay_ms: body[frame_idx].delay,
            parts,
        });
    }

    frames
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
    job_catalog: Option<Res<crate::character::job::JobCatalog>>,
) {
    let Some(job_catalog) = job_catalog else {
        warn!("spawn_character: JobCatalog not ready yet, skipping");
        return;
    };
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

// ── Action loading (two-phase: kick off, then combine when assets ready) ──

pub fn on_set_action(
    trigger: On<SetAction>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut query: Query<
        (
            Entity,
            &mut CharacterActionAnimation,
            &CharacterConfig,
            &mut CharacterFaceAnimation,
        ),
        (With<CharacterRoot>, Without<PendingActionLoad>),
    >,
) {
    let ev = trigger.event();
    let Ok((root, mut anim, config, mut face_anim)) = query.get_mut(ev.entity) else {
        return;
    };

    if anim.return_to_default {
        anim.pending_action = Some(PendingCharacterAction::Action {
            action: ev.action.clone(),
            return_to_default: ev.return_to_default,
        });
        return;
    }

    // Build asset paths
    let body_path = format!("wz://char-body/{}/{}.charbody", config.skin_suffix, ev.action);
    let body_handle = asset_server.load::<WzCharBodyAsset>(&body_path);

    let hair_path = format!("wz://char-hair/{}/{}.charhair", config.hair_id, ev.action);
    let hair_handle = asset_server.load::<WzHairBodyAsset>(&hair_path);

    let equip_handles: Vec<Handle<WzEquipActionAsset>> = config
        .equipment
        .iter()
        .map(|(_slot, item_id)| {
            let path = format!("wz://char-equip/{}/{}.charequip", item_id, ev.action);
            asset_server.load::<WzEquipActionAsset>(&path)
        })
        .collect();

    let face_path = format!("wz://char-face/{}/{}.charface", config.face_id, face_anim.expression);
    let face_handle = asset_server.load::<WzFaceExpressionAsset>(&face_path);

    // Store pending load handles
    commands.entity(root).insert(PendingActionLoad {
        body_handle,
        hair_handle: Some(hair_handle),
        equip_handles,
        face_handle: Some(face_handle),
        action: ev.action.clone(),
    });

    anim.action = ev.action.clone();
    anim.return_to_default = ev.return_to_default;
}

pub fn process_pending_action_load(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    zmap: Option<Res<ZMap>>,
    body_assets: Res<Assets<WzCharBodyAsset>>,
    hair_assets: Res<Assets<WzHairBodyAsset>>,
    equip_assets: Res<Assets<WzEquipActionAsset>>,
    face_assets: Res<Assets<WzFaceExpressionAsset>>,
    mut query: Query<
        (
            Entity,
            &mut PendingActionLoad,
            &CharacterConfig,
            &mut CharacterActionAnimation,
            Option<&Children>,
            &mut CharacterLabels,
            &mut CharacterFaceAnimation,
        ),
        With<CharacterRoot>,
    >,
    body_query: Query<&Children, With<CharacterBody>>,
    part_query: Query<&PartName>,
) {
    let Some(zmap) = zmap else { return };

    for (
        root,
        pending,
        config,
        mut anim,
        children,
        mut labels,
        mut face_anim,
    ) in &mut query
    {
        // Check all assets ready
        let body = match body_assets.get(&pending.body_handle) {
            Some(b) => b,
            None => continue,
        };

        let hair = match &pending.hair_handle {
            Some(h) => match hair_assets.get(h) {
                Some(h) => Some(h),
                None => continue,
            },
            None => None,
        };

        let mut equips = Vec::new();
        for handle in &pending.equip_handles {
            match equip_assets.get(handle) {
                Some(e) => equips.push(e),
                None => continue, // not all ready yet
            }
        }
        if equips.len() != pending.equip_handles.len() {
            continue;
        }

        let face = match &pending.face_handle {
            Some(f) => match face_assets.get(f) {
                Some(f) => Some(f),
                None => continue,
            },
            None => None,
        };

        // All assets are ready — combine
        info!("All character action assets loaded for '{}', combining", pending.action);

        // Apply zmap resolution
        let mut body_frames = body.frames.clone();
        let mut hair_frames = hair.map(|h| h.frames.clone()).unwrap_or_default();
        resolve_z_frames(&mut body_frames, &zmap);
        resolve_z_frames(&mut hair_frames, &zmap);

        let equip_frames: Vec<Vec<wz::BodyFrame>> = equips
            .iter()
            .map(|e| {
                let mut frames = e.frames.clone();
                resolve_z_frames(&mut frames, &zmap);
                frames
            })
            .collect();

        let face_part = face.and_then(|f| {
            f.frames.first().map(|ff| {
                let mut part = ff.part.clone();
                if let Some(slot) = &part.slot {
                    part.z = zmap.depth(slot);
                }
                part
            })
        });

        let action_frames = combine_action_frames(
            &asset_server,
            &body_frames,
            &hair_frames,
            &equip_frames,
            face_part.as_ref(),
        );

        // Apply to entity
        let default_children = Children::default();
        let children_ref = children.unwrap_or(&default_children);
        let (body_entity, body_children) = match children_ref
            .iter()
            .find_map(|child| body_query.get(child).ok().map(|c| (child, c)))
        {
            Some((entity, children)) => (Some(entity), children),
            None => (None, &default_children),
        };

        let returned_face = transform_action_to_entities(
            &mut commands,
            &action_frames,
            root,
            body_entity,
            body_children,
            &part_query,
        );

        if face_anim.face_entity.is_none() {
            face_anim.face_entity = returned_face;
        }

        anim.frame_idx = 0;
        anim.frame_count = action_frames.len();
        if let Some(first) = action_frames.first() {
            let secs = (first.delay_ms as f32 / 1000.0).max(MIN_TIMER_SECS);
            anim.timer = Timer::from_seconds(secs, TimerMode::Repeating);
        }
        info!(
            "[char action] combined '{}' | {} frames",
            pending.action,
            action_frames.len(),
        );

        labels.action.clone_from(&pending.action);

        // Remove pending
        commands.entity(root).remove::<PendingActionLoad>();
    }
}

// ── Face expression loading ──

pub fn on_set_face_expression(
    trigger: On<SetFaceExpression>,
    mut commands: Commands,
    mut query: Query<
        (&CharacterConfig, &mut CharacterFaceAnimation, Option<&Children>),
        With<CharacterRoot>,
    >,
    asset_server: Res<AssetServer>,
) {
    let ev = trigger.event();
    let Ok((config, mut face_anim, _root_children)) = query.get_mut(ev.entity) else {
        warn!("[on_set_face_expression] failed to query entity");
        return;
    };

    face_anim.expression = ev.expression.clone();
    face_anim.frame_idx = 0;

    // Load face expression data via asset
    let face_path = format!("wz://char-face/{}/{}.charface", config.face_id, ev.expression);
    let face_handle = asset_server.load::<WzFaceExpressionAsset>(&face_path);

    commands.entity(ev.entity).insert(PendingFaceLoad(face_handle));
}

pub fn process_pending_face_load(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    face_assets: Res<Assets<WzFaceExpressionAsset>>,
    mut query: Query<
        (
            Entity,
            &mut CharacterFaceAnimation,
            &PendingFaceLoad,
        ),
        With<CharacterRoot>,
    >,
    mut part_query: Query<(
        &PartName,
        &mut Sprite,
        &mut Visibility,
        &mut Anchor,
    )>,
) {
    for (entity, mut face_anim, pending_face) in &mut query {
        let Some(face_expr) = face_assets.get(&pending_face.0) else {
            continue;
        };

        // Convert face expression frames to FaceFrame with image handles
        let mut frames = Vec::new();
        for f in &face_expr.frames {
            let image = load_part_image(&asset_server, &f.part.image_path);
            frames.push(FaceFrame {
                image,
                anchor: Vec2::new(-0.5, -0.5),
                delay_ms: f.delay,
            });
        }
        face_anim.frames = frames;
        face_anim.frame_idx = 0;

        // Apply first frame
        if let Some(first) = face_anim.frames.first().cloned() {
            let delay_secs = (first.delay_ms as f32 / 1000.0).max(MIN_TIMER_SECS);
            face_anim.timer = Timer::from_seconds(delay_secs, TimerMode::Repeating);

            if let Some(fe) = face_anim.face_entity {
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

        // Remove the face handle
        commands.entity(entity).remove::<PendingFaceLoad>();
        info!("[char face] loaded '{}' with {} frames", face_anim.expression, face_anim.frames.len());
    }
}
