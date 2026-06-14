use bevy::prelude::*;
use bevy::sprite::{Anchor, Text2dShadow};
use std::collections::{HashMap, HashSet};

use crate::character::components::*;
use crate::character::events::*;
use crate::character::job::{Job, JobCatalog};
use crate::character::skills::{LearnedSkills, SkillDatabase, SkillEffect, SkillEffectRoot};
use crate::character::types::*;
use crate::input::{IsLocalPlayer, KeyAction};
use crate::layer::GameLayer;
use crate::physics::PhysicsState;

pub const DEFAULT_CHARACTER_ACTION: &str = "stand1";
const MIN_TIMER_SECS: f32 = 0.016;

fn zig_zag(frame: usize, count: usize) -> usize {
    if count <= 1 { return 0; }
    let period = 2 * (count - 1);
    let pos = frame % period;
    if pos < count { pos } else { period - pos }
}

/// Two action lists discovered from WZ: basic (common) and composite (skill-specific).
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct ActionLists {
    pub basic: Vec<String>,
    pub composite: Vec<String>,
}

impl Default for ActionLists {
    fn default() -> Self {
        ActionLists {
            basic: Vec::new(),
            composite: Vec::new(),
        }
    }
}

pub fn load_action_lists(wz: &wz::WzData) -> ActionLists {
    let basic: Vec<String> = wz
        .list_children("Character/00002001.img")
        .unwrap_or_default()
        .into_iter()
        .filter(|a| a != "info")
        .collect();
    let basic_set: HashSet<&str> = basic.iter().map(|s| s.as_str()).collect();
    let all = wz
        .list_children("Character/00002000.img")
        .unwrap_or_default()
        .into_iter()
        .filter(|a| a != "info");
    let composite: Vec<String> = all.filter(|a| !basic_set.contains(a.as_str())).collect();
    info!(
        "ActionLists: {} basic, {} composite actions",
        basic.len(),
        composite.len()
    );
    ActionLists { basic, composite }
}

#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct ActionCycle {
    pub basic: usize,
    pub composite: usize,
    pub skill: usize,
}

const LABEL_Y: f32 = -48.0;
const LABEL_GAP: f32 = 4.0;
const LABEL_Z_OFFSET: f32 = 240.0;

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
    info!("[collect_raw_parts] frame_idx={} parts: {:?}", frame_idx, raw_parts.keys().collect::<Vec<_>>());
    raw_parts
}

fn compute_body_part_positions<'a>(
    raw_parts: &'a HashMap<&'a str, &'a wz::BodyPart>,
) -> HashMap<&'a str, Vec3> {
    const PART_ORDER: &[&str] = &["body", "head", "arm"];

    let mut sorted: Vec<(&str, &wz::BodyPart)> = raw_parts.iter().map(|(&k, &v)| (k, v)).collect();
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
    let face = wz.load_face_expression(config.face_id, face_expression)
        .ok()
        .and_then(|fe| fe.frames.first().map(|f| {
            let mut part = f.part.clone();
            if let Some(slot) = &part.slot {
                part.z = zmap.depth(slot);
            }
            part
        }));

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
            if part_name == "face" {
                info!("[load_action_frames] face: img_path='{}' origin=({},{}) img={}x{} anchor=({:.3},{:.3})",
                    part.image_path, part.origin.0, part.origin.1, w, h, anchor.x, anchor.y);
            }
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
    Vec2::new(origin.0 / img_width - 0.5, origin.1 / img_height - 0.5)
}

fn build_action_label(commands: &mut Commands, action: &str) -> Entity {
    commands
        .spawn((
            Name::new("ActionLabel"),
            CharacterActionLabel,
            Text2d::new(action),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::WHITE),
            TextLayout::justify(Justify::Right),
            Anchor::TOP_RIGHT,
            TextBackgroundColor(Color::BLACK.with_alpha(0.55)),
            Text2dShadow {
                offset: Vec2::new(1.0, -1.0),
                color: Color::BLACK,
            },
            Transform::from_translation(Vec3::new(
                -LABEL_GAP,
                LABEL_Y,
                GameLayer::Character.with_offset(LABEL_Z_OFFSET),
            )),
        ))
        .id()
}

fn build_job_label(commands: &mut Commands, label: &str) -> Entity {
    commands
        .spawn((
            Name::new("JobLabel"),
            CharacterJobLabel,
            Text2d::new(label),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.88, 0.45)),
            TextLayout::justify(Justify::Left),
            Anchor::TOP_LEFT,
            TextBackgroundColor(Color::BLACK.with_alpha(0.55)),
            Text2dShadow {
                offset: Vec2::new(1.0, -1.0),
                color: Color::BLACK,
            },
            Transform::from_translation(Vec3::new(
                LABEL_GAP,
                LABEL_Y,
                GameLayer::Character.with_offset(LABEL_Z_OFFSET),
            )),
        ))
        .id()
}

fn transform_action_to_entities(
    commands: &mut Commands,
    frames: &[ActionFrame],
    character_root: Entity,
    character_body: Option<Entity>,
    body_children: &Children,
    part_query: &Query<&PartName>,
) -> Option<Entity> {
    commands.entity(character_root).insert(CurrentAction {
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
        .filter_map(|child| part_query.get(child).ok().map(|pn| (pn.0.clone(), child)))
        .collect();

    if let Some(body_entity) = character_body {
        existing_body_entities.insert("body".to_string(), body_entity);
    };

    let first_frame = &frames[0];
    let mut current_body_entity = character_body;
    let mut face_entity = None;

    info!("[transform_action] {} parts to spawn: {:?}", part_names.len(), part_names);

    for name in &part_names {
        let pose = first_frame
            .parts
            .get(name)
            .cloned()
            .unwrap_or_else(PartPose::hidden);

        let entity = if let Some(&e) = existing_body_entities.get(name) {
            info!("[transform_action] reusing existing entity for part '{}': {:?}", name, e);
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
                info!("[transform_action] spawned body entity: {:?} as child of root {:?}", e, character_root);
            } else {
                let body_entity = current_body_entity.expect("body must be spawned first");
                commands.entity(body_entity).add_child(e);
                info!("[transform_action] spawned part '{}' entity: {:?} as child of body {:?}", name, e, body_entity);
            }
            e
        };
        if name == "face" {
            face_entity = Some(entity);
            info!("[transform_action] face entity: {:?}", entity);
        }
        info!("[transform_action] part '{}' pos=({:.1},{:.1},{:.1}) anchor=({:.3},{:.3})",
            name, pose.position.x, pose.position.y, pose.position.z, pose.anchor.x, pose.anchor.y);
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
    job_catalog: Res<JobCatalog>,
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

    let action_label = build_action_label(&mut commands, &ev.action);
    commands.entity(root).add_child(action_label);
    let job_label_text = job_catalog.display_label(ev.config.job);
    let job_label = build_job_label(&mut commands, &job_label_text);
    commands.entity(root).add_child(job_label);

    commands.entity(root).insert(CharacterLabels {
        action: action_label,
        job: job_label,
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
            &Children,
            &CharacterLabels,
            &mut CharacterFaceAnimation,
        ),
        With<CharacterRoot>,
    >,
    body_query: Query<&Children, With<CharacterBody>>,
    part_query: Query<&PartName>,
    mut label_query: Query<&mut Text2d, With<CharacterActionLabel>>,
) {
    let ev = trigger.event();
    let root = ev.entity;
    let Ok((character_root, mut anim, config, children, labels, mut face_anim)) =
        query.get_mut(root)
    else {
        return;
    };

    let default_children = Children::default();
    let (body_entity, body_children) = match children
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

    let action_frames = load_action_frames(&wz, &mut images, &zmap, config, &ev.action, &face_anim.expression);

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

    if let Ok(mut label) = label_query.get_mut(labels.action) {
        label.0.clone_from(&ev.action);
    }
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
    mut query: Query<(&CharacterConfig, &mut CharacterFaceAnimation, &Children), With<CharacterRoot>>,
    body_query: Query<&Children, With<CharacterBody>>,
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
    info!("[on_set_face_expression] triggered for entity {:?} expression='{}'", ev.entity, ev.expression);
    let Ok((config, mut face_anim, root_children)) = query.get_mut(ev.entity) else {
        warn!("[on_set_face_expression] failed to query entity");
        return;
    };

    face_anim.expression = ev.expression.clone();
    face_anim.frame_idx = 0;
    face_anim.frames = load_face_expression_image_frames(&wz, &mut images, config.face_id, &ev.expression);
    info!("[on_set_face_expression] loaded {} face frames, face_entity={:?}", face_anim.frames.len(), face_anim.face_entity);

    let first = face_anim.frames.first().cloned();
    let face_entity = face_anim.face_entity;
    if let Some(first) = first {
        let delay_secs = (first.delay_ms as f32 / 1000.0).max(MIN_TIMER_SECS);
        face_anim.timer = Timer::from_seconds(delay_secs, TimerMode::Repeating);

        if let Some(fe) = face_entity {
            info!("[on_set_face_expression] applying first frame to entity {:?}", fe);
            match part_query.get_mut(fe) {
                Ok((pn, mut sprite, mut visibility, mut anchor)) => {
                    sprite.image = first.image;
                    *anchor = Anchor(first.anchor);
                    *visibility = Visibility::Visible;
                    info!("[on_set_face_expression] SUCCESS: part='{}' visibility=Visible anchor=({:.3},{:.3})", pn.0, anchor.0.x, anchor.0.y);
                }
                Err(e) => {
                    warn!("[on_set_face_expression] FAILED to get face entity {:?}: {:?}", fe, e);
                }
            }
        } else {
            warn!("[on_set_face_expression] face_entity is None!");
        }
    } else {
        warn!("[on_set_face_expression] no face frames loaded!");
    }
}

// ── Animation ──

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
        let total_frames = if anim.frame_count > 1 { 2 * (anim.frame_count - 1) } else { 1 };
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
            transform.translation = Vec3::new(pose.position.x, pose.position.y, pose.position.z);
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
    face_state.timer = Timer::from_seconds(delay_secs.max(MIN_TIMER_SECS), TimerMode::Repeating);

    frame
}

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
    for (mut face_state, children) in &mut query {
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
    for (phys, mut anim, entity) in &mut query {
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
        let facing_left = if phys.left { true } else { false };
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
    skill_db: Res<SkillDatabase>,
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
            Transform::from_translation(Vec3::new(0.0, 0.0, GameLayer::Skill.with_offset(0.0))),
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

    let label = commands
        .spawn((
            Name::new("SkillLabel"),
            SkillNameLabel,
            Text2d::new(skill.name.clone()),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::WHITE),
            TextLayout::justify(Justify::Center),
            Anchor::TOP_CENTER,
            TextBackgroundColor(Color::BLACK.with_alpha(0.55)),
            Text2dShadow {
                offset: Vec2::new(1.0, -1.0),
                color: Color::BLACK,
            },
            Transform::from_translation(Vec3::new(0.0, -60.0, GameLayer::Skill.with_offset(0.0))),
        ))
        .id();
    commands.entity(effect_root).add_child(label);
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
    }
}

// ── Input ──

fn resolve_action_from_key(
    key: KeyAction,
    cycle: &mut ActionCycle,
    lists: &ActionLists,
) -> Option<String> {
    match key {
        KeyAction::Jump | KeyAction::JumpAction => Some("jump".to_string()),
        KeyAction::CycleBasic if !lists.basic.is_empty() => {
            let idx = cycle.basic % lists.basic.len();
            cycle.basic = (cycle.basic + 1) % lists.basic.len();
            Some(lists.basic[idx].clone())
        }
        KeyAction::CycleComposite if !lists.composite.is_empty() => {
            let idx = cycle.composite % lists.composite.len();
            cycle.composite = (cycle.composite + 1) % lists.composite.len();
            Some(lists.composite[idx].clone())
        }
        _ => None,
    }
}

pub fn on_character_action(
    trigger: On<crate::input::ActionEvent>,
    mut query: Query<
        (
            Entity,
            &mut Job,
            &mut CharacterConfig,
            &CharacterLabels,
            &mut CharacterActionAnimation,
        ),
        (With<CharacterRoot>, With<IsLocalPlayer>),
    >,
    mut cycle: ResMut<ActionCycle>,
    action_lists: Res<ActionLists>,
    skill_db: Res<SkillDatabase>,
    job_catalog: Res<JobCatalog>,
    mut job_label_query: Query<&mut Text2d, With<CharacterJobLabel>>,
    mut commands: Commands,
) {
    let (entity, mut job, mut config, labels, mut current_anim) = match query.iter_mut().next() {
        Some(result) => result,
        None => return,
    };

    let key = trigger.event().0;

    // Pure resolution: key → action name (no side effects)
    if let Some(action) = resolve_action_from_key(key, &mut cycle, &action_lists) {
        let return_to_default = action != DEFAULT_CHARACTER_ACTION;
        if current_anim.return_to_default {
            current_anim.pending_action = Some(PendingCharacterAction::Action {
                action,
                return_to_default,
            });
        } else {
            commands.trigger(SetAction {
                entity,
                action,
                return_to_default,
            });
        }
        return;
    }

    // Side-effect actions
    match key {
        KeyAction::CycleJob => {
            let Some(next_job) = job_catalog.next_after(*job) else {
                return;
            };
            *job = next_job;
            config.job = next_job;
            let label = job_catalog.display_label(next_job);
            if let Ok(mut job_label) = job_label_query.get_mut(labels.job) {
                job_label.0.clone_from(&label);
            }
        }
        KeyAction::CycleSkill => {
            let active: Vec<_> = skill_db
                .active_skills_for_job(&job.lineage())
                .into_iter()
                .filter(|s| !s.effect_frames.is_empty())
                .collect();
            if active.is_empty() {
                return;
            }
            let idx = cycle.skill % active.len();
            cycle.skill = (cycle.skill + 1) % active.len();
            let skill_entry = active[idx];
            if current_anim.return_to_default {
                current_anim.pending_action = Some(PendingCharacterAction::Skill {
                    skill_id: skill_entry.id,
                });
            } else {
                commands.trigger(UseSkill {
                    entity,
                    skill_id: skill_entry.id,
                });
            }
        }
        _ => {}
    }
}
