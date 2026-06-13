use bevy::prelude::*;
use bevy::sprite::{Anchor, Text2dShadow};
use std::collections::HashMap;

use crate::character::components::*;
use crate::character::events::*;
use crate::character::job::{Job, JobCatalog};
use crate::character::skills::{
    LearnedSkills, SkillDatabase, SkillEffect, SkillEffectRoot,
};
use crate::character::types::*;
use crate::input::{CharacterIntent, IsLocalPlayer, KeyAction};
use crate::layer::GameLayer;
use crate::physics::PhysicsState;

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

pub const DEFAULT_CHARACTER_ACTION: &str = "stand1";

/// Action lists per category, in the order they appear in WZ.
pub const STANCE_ACTIONS: &[&str] = &[
    "stand1", "stand2", "walk1", "walk2", "sit", "prone", "prone2", "ladder", "ladder2", "rope",
    "rope2", "fly", "float",
];

pub const ALERT_ACTIONS: &[&str] = &["alert", "alert2", "alert3", "alert4", "alert5", "alert6"];

pub const SWING_ACTIONS: &[&str] = &[
    "swingO1",
    "swingO2",
    "swingO3",
    "swingOF",
    "swingP1",
    "swingP2",
    "swingPF",
    "swingT1",
    "swingT2",
    "swingT3",
    "swingTF",
    "swingT2PoleArm",
    "swingP1PoleArm",
    "swingP2PoleArm",
];

pub const STAB_ACTIONS: &[&str] = &[
    "stabO1",
    "stabO2",
    "stabOF",
    "stabT1",
    "stabT2",
    "stabTF",
    "proneStab",
];

pub const MULTI_SWING_ACTIONS: &[&str] = &[
    "doubleSwing",
    "tripleSwing",
    "overSwingDouble",
    "overSwingTriple",
    "fullSwingDouble",
    "fullSwingTriple",
];

pub const RANGED_ACTIONS: &[&str] = &[
    "shoot1", "shoot2", "shoot6", "shootF", "shot", "handgun", "cannon", "torpedo",
];

pub const MAGIC_ACTIONS: &[&str] = &[
    "magic1",
    "magic2",
    "magic3",
    "magic4",
    "magic5",
    "magicmissile",
    "superMagicmissile",
    "heal",
    "holyShield",
    "genesis",
    "sanctuary",
    "resurrection",
    "timeleap",
    "meteor",
    "blizzard",
    "chainlightning",
    "firestrike",
    "fireburner",
    "flamegear",
    "icebreathe_prepare",
    "breathe_prepare",
    "shockwave",
    "windspear",
    "windshot",
    "stormbreak",
    "wave",
    "magicFlare",
    "magicBooster",
    "coolingeffect",
    "souldriver",
];

pub const MOVEMENT_SKILL_ACTIONS: &[&str] = &[
    "dash",
    "combatStep",
    "straight",
    "somersault",
    "rollingSpin",
    "backstep",
    "rush",
    "rush2",
    "ghostfly",
    "ghostjump",
    "ghostladder",
    "ghostproneStab",
    "ghostrope",
    "ghostsit",
    "ghoststand",
    "ghostwalk",
];

const LABEL_Y: f32 = -48.0;
const LABEL_GAP: f32 = 4.0;
const LABEL_Z_OFFSET: f32 = 240.0;

fn resolve_equipment(
    wz: &wz::WzData,
    equipment: &[(EquipSlot, u32)],
) -> Vec<EquipmentEntry> {
    equipment
        .iter()
        .map(|(slot, item_id)| {
            let equip_result = wz.load_equip(*item_id as i32);
            let vslot = match &equip_result {
                Ok(e) => {
                    let num_actions = e.actions.len();
                    let action_keys: Vec<_> = e.actions.keys().cloned().collect();
                    info!(
                        "  equip item_id={} slot={:?} loaded OK: {} actions {:?} vslot={:?}",
                        item_id, slot, num_actions, action_keys, e.info.vslot,
                    );
                    e.info.vslot.as_ref().map(|s| split_vslot(s)).unwrap_or_default()
                }
                Err(e) => {
                    warn!("  equip item_id={} slot={:?} FAILED: {e}", item_id, slot);
                    Vec::new()
                }
            };
            EquipmentEntry {
                slot: *slot,
                item_id: *item_id,
                vslot,
            }
        })
        .collect()
}

fn part_translation(layer: &SpriteLayer, pos: Vec3, facing_left: bool) -> Vec3 {
    if !facing_left {
        // Mirror the origin (connection point) around root center on the x axis:
        //   origin_root = pos + origin
        //   new_origin_root.x = -origin_root.x
        //   new_pos.x + origin.x = -(pos.x + origin.x)
        //   new_pos.x = -pos.x - 2 * origin.x
        Vec3::new(-pos.x - 2.0 * layer.origin.x, pos.y, pos.z)
    } else {
        pos
    }
}

fn build_part_entity(
    commands: &mut Commands,
    layer: &SpriteLayer,
    pos: Vec3,
    facing_left: bool,
) -> Entity {
    let mut sprite = Sprite::from_image(layer.image.clone());
    sprite.flip_x = !facing_left;
    commands
        .spawn((
            sprite,
            Transform::from_translation(part_translation(layer, pos, facing_left)),
            CharacterPart {
                layer: layer.layer_name.clone(),
                z_base: layer.z,
            },
        ))
        .id()
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

fn update_part_sprite(
    part_entity: Entity,
    layer: &SpriteLayer,
    pos: Vec3,
    facing_left: bool,
    part_query: &mut Query<(&mut Sprite, &mut Transform, &CharacterPart)>,
) {
    if let Ok((mut sprite, mut transform, _)) = part_query.get_mut(part_entity) {
        sprite.image = layer.image.clone();
        sprite.flip_x = !facing_left;
        transform.translation = part_translation(layer, pos, facing_left);
        transform.scale = Vec3::ONE;
    }
}

fn apply_current_frame(
    anim: &CharacterAnimation,
    frame_data: &CharacterFrameData,
    part_entities: &PartEntities,
    part_query: &mut Query<(&mut Sprite, &mut Transform, &CharacterPart)>,
    body_origin: &mut Option<Vec2>,
) {
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

    let (positions, _) = compute_frame_transforms(&merged_parts, GameLayer::Character.base_z());

    *body_origin = merged_parts
        .iter()
        .find(|p| p.layer_name == "body")
        .map(|body| body.origin);

    for layer in &merged_parts {
        if let Some(&child) = part_entities.map.get(&layer.layer_name) {
            let pos = positions
                .get(&layer.layer_name)
                .copied()
                .unwrap_or(Vec3::ZERO);
            update_part_sprite(child, layer, pos, anim.facing_left, part_query);
        }
    }
}

pub fn spawn_character(
    trigger: On<SpawnCharacter>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    zmap: Res<ZMap>,
    slot_map: Res<SlotMap>,
    job_catalog: Res<JobCatalog>,
) {
    let ev = trigger.event();
    info!("spawn_character at {:?}", ev.transform.translation);
    let wz = wz::WzData::global();

    let loaded = wz.load_character(ev.config.skin_suffix, ev.config.hair_id, ev.config.face_id)
        .expect("character data");

    info!(
        "character loaded: skin_suffix={} hair_id={} face_id={} body_actions={} face_expressions={}",
        ev.config.skin_suffix, ev.config.hair_id, ev.config.face_id,
        loaded.body.len(), loaded.face_expressions.len(),
    );
    for (act, frames) in &loaded.body {
        let part_names: Vec<_> = frames.first().map(|f| f.parts.iter().map(|p| &p.layer_name).collect::<Vec<_>>()).unwrap_or_default();
        info!("  body action '{}': {} frames, parts={:?}", act, frames.len(), part_names);
    }
    for (act, frames) in &loaded.face_expressions {
        info!("  face expression '{}': {} frames", act, frames.len());
    }

    let equipment_entries = resolve_equipment(wz, &ev.config.equipment);

    // Convert wz::FrameData to client FrameData by resolving images
    fn convert_frames(
        wz_frames: &[wz::FrameData],
        images: &mut Assets<Image>,
        zmap: &ZMap,
        slot_map: &SlotMap,
        equip_slot: Option<EquipSlot>,
    ) -> Vec<FrameData> {
        wz_frames.iter().map(|f| {
            let parts = f.parts.iter().map(|p| {
                let handle = load_character_image(&p.image_path, images);
                let origin = Vec2::new(p.origin.0, p.origin.1);
                let z = zmap.depth(p.slot.as_deref().unwrap_or(""));
                let slot = p.slot.as_ref()
                    .and_then(|z_str| slot_map.slot_for(z_str))
                    .map(String::from);
                let source = match p.source {
                    wz::PartSource::Body => PartSource::Body,
                    wz::PartSource::Head => PartSource::Head,
                    wz::PartSource::Hair => PartSource::Hair,
                    wz::PartSource::Face => PartSource::Face,
                    wz::PartSource::Equipment => PartSource::Equipment(equip_slot.unwrap_or(EquipSlot::Cap)),
                };
                let map = p.map.iter().map(|(k, v)| (k.clone(), Vec2::new(v.0, v.1))).collect();
                SpriteLayer { image: handle, origin, map, z, layer_name: p.layer_name.clone(), slot, source }
            }).collect();
            FrameData { parts, delay: f.delay }
        }).collect()
    }

    fn convert_actions(
        wz_actions: &std::collections::HashMap<String, Vec<wz::FrameData>>,
        images: &mut Assets<Image>,
        zmap: &ZMap,
        slot_map: &SlotMap,
        equip_slot: Option<EquipSlot>,
    ) -> std::collections::HashMap<String, Vec<FrameData>> {
        wz_actions.iter().map(|(k, v)| {
            (k.clone(), convert_frames(v, images, zmap, slot_map, equip_slot))
        }).collect()
    }

    fn merge_actions(
        target: &mut std::collections::HashMap<String, Vec<FrameData>>,
        source: &std::collections::HashMap<String, Vec<FrameData>>,
    ) {
        for (action_name, source_frames) in source {
            if let Some(target_frames) = target.get_mut(action_name) {
                for (i, target_frame) in target_frames.iter_mut().enumerate() {
                    let src_idx = i % source_frames.len().max(1);
                    if let Some(src_frame) = source_frames.get(src_idx) {
                        target_frame.parts.extend(src_frame.parts.clone());
                    }
                }
            }
        }
    }

    fn filter_actions(
        actions: std::collections::HashMap<String, Vec<FrameData>>,
        equipment: &[EquipmentEntry],
    ) -> std::collections::HashMap<String, Vec<FrameData>> {
        let filter_frame = |frame: FrameData| FrameData {
            parts: filter_hidden_sprites(frame.parts, equipment),
            delay: frame.delay,
        };
        actions.into_iter()
            .map(|(name, frames)| (name, frames.into_iter().map(filter_frame).collect()))
            .collect()
    }

    let mut actions = convert_actions(&loaded.body, &mut images, &zmap, &slot_map, None);

    let head_actions = convert_actions(&loaded.head, &mut images, &zmap, &slot_map, None);
    merge_actions(&mut actions, &head_actions);
    let hair_actions = convert_actions(&loaded.hair, &mut images, &zmap, &slot_map, None);
    merge_actions(&mut actions, &hair_actions);

    let mut face_expressions = convert_actions(&loaded.face_expressions, &mut images, &zmap, &slot_map, None);

    for eq_entry in &equipment_entries {
        if let Ok(equip_data) = wz.load_equip(eq_entry.item_id as i32) {
            info!(
                "  equip {} slot={:?}: {} actions, vslot={:?}",
                eq_entry.item_id, eq_entry.slot, equip_data.actions.len(), equip_data.info.vslot,
            );
            let equip_actions = convert_actions(&equip_data.actions, &mut images, &zmap, &slot_map, Some(eq_entry.slot));
            merge_actions(&mut actions, &equip_actions);
        } else {
            warn!("  equip {} slot={:?}: failed to load EquipData", eq_entry.item_id, eq_entry.slot);
        }
    }

    actions = filter_actions(actions, &equipment_entries);
    face_expressions = filter_actions(face_expressions, &equipment_entries);

    info!(
        "spawn_character: {} equipment entries, {} actions after merge+filter, action '{}' present={}",
        equipment_entries.len(),
        actions.len(),
        ev.action,
        actions.contains_key(&ev.action),
    );

    let face_face_frames = face_expressions
        .get(&ev.face_expression)
        .cloned()
        .unwrap_or_default();

    let first_frames = actions.get(&ev.action);
    let parts = first_frames
        .and_then(|frames| frames.first())
        .map(|f| &f.parts)
        .cloned()
        .unwrap_or_default();

    let mut merged_parts = parts.clone();
    if let Some(face_frame) = face_face_frames.first() {
        merged_parts.extend(face_frame.parts.clone());
    }

    info!(
        "spawn_character: {} merged parts for action '{}'",
        merged_parts.len(),
        ev.action,
    );
    for p in &merged_parts {
        info!("  part: '{}' z={} source={:?}", p.layer_name, p.z, p.source);
    }
    let (positions, parents) =
        compute_frame_transforms(&merged_parts, GameLayer::Character.base_z());

    let delay = first_frames
        .and_then(|frames| frames.first())
        .map(|f| f.delay)
        .unwrap_or(100);
    let facing_left = true;

    let pos = ev.transform.translation;

    let body_origin = merged_parts
        .iter()
        .find(|p| p.layer_name == "body")
        .map(|body| body.origin)
        .unwrap_or(Vec2::ZERO);

    let root = commands
        .spawn((
            Name::new("Character"),
            CharacterRoot { body_origin },
            ev.config.clone(),
            CharacterEquipment {
                entries: equipment_entries,
            },
            CharacterAnimation {
                action: ev.action.clone(),
                default_action: DEFAULT_CHARACTER_ACTION.to_string(),
                return_to_default: false,
                pending_action: None,
                frame_idx: 0,
                timer: Timer::from_seconds(delay as f32 / 1000.0, TimerMode::Repeating),
                face_expression: ev.face_expression.clone(),
                face_frame_idx: 0,
                face_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
                facing_left,
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

    let action_label = build_action_label(&mut commands, &ev.action);
    commands.entity(root).add_child(action_label);
    let job_label_text = job_catalog.display_label(ev.config.job);
    let job_label = build_job_label(&mut commands, &job_label_text);
    commands.entity(root).add_child(job_label);

    let mut part_map = HashMap::new();
    let mut part_entities: Vec<(Entity, Option<String>)> = Vec::new();

    for layer in &merged_parts {
        let pos = positions
            .get(&layer.layer_name)
            .copied()
            .unwrap_or(Vec3::new(0.0, 0.0, GameLayer::Character.base_z() + layer.z));
        let parent_name = parents.get(&layer.layer_name).and_then(|p| p.clone());
        info!(
            "spawn part: '{}' pos=({:.1},{:.1},{:.1}) parent={:?} z_offset={} source={:?} origin=({:.1},{:.1})",
            layer.layer_name, pos.x, pos.y, pos.z, parent_name, layer.z, layer.source, layer.origin.x, layer.origin.y,
        );
        let child = build_part_entity(&mut commands, layer, pos, facing_left);
        part_map.insert(layer.layer_name.clone(), child);
        part_entities.push((child, parent_name));
    }

    for (child, parent_name) in &part_entities {
        match parent_name {
            Some(pname) => {
                if let Some(parent_entity) = part_map.get(pname) {
                    commands.entity(*parent_entity).add_child(*child);
                }
            }
            None => {
                commands.entity(root).add_child(*child);
            }
        }
    }
    commands.entity(root).insert(PartEntities { map: part_map });
}

fn load_character_image(path: &str, images: &mut Assets<Image>) -> Handle<Image> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let wz = wz::WzData::global();
    let dynamic_image = wz.load_image(path).unwrap_or_else(|e| {
        panic!("failed to load character image at {path}: {e}")
    });
    let rgba = dynamic_image.to_rgba8();
    let (width, height) = rgba.dimensions();
    images.add(Image::new(
        Extent3d { width, height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        rgba.into_raw(),
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    ))
}

pub fn on_set_facing(
    trigger: On<SetFacing>,
    mut query: Query<(&mut CharacterAnimation, &CharacterFrameData, &PartEntities, &mut CharacterRoot)>,
    mut part_query: Query<(&mut Sprite, &mut Transform, &CharacterPart)>,
) {
    let ev = trigger.event();
    let Ok((mut anim, frame_data, part_entities, mut char_root)) = query.get_mut(ev.entity) else {
        return;
    };

    anim.facing_left = ev.facing_left;
    let mut body_origin = Some(char_root.body_origin);
    apply_current_frame(&anim, frame_data, part_entities, &mut part_query, &mut body_origin);
    if let Some(origin) = body_origin {
        char_root.body_origin = origin;
    }
}

pub fn update_character_facing_from_intent(
    mut query: Query<
        (
            &CharacterIntent,
            &mut CharacterAnimation,
            &CharacterFrameData,
            &PartEntities,
            &mut CharacterRoot,
        ),
        (With<CharacterRoot>, With<IsLocalPlayer>),
    >,
    mut part_query: Query<(&mut Sprite, &mut Transform, &CharacterPart)>,
) {
    for (intent, mut anim, frame_data, part_entities, mut char_root) in &mut query {
        let facing_left = match (intent.left, intent.right) {
            (true, false) => true,
            (false, true) => false,
            _ => continue,
        };
        if anim.facing_left == facing_left {
            continue;
        }

        anim.facing_left = facing_left;
        let mut body_origin = Some(char_root.body_origin);
        apply_current_frame(&anim, frame_data, part_entities, &mut part_query, &mut body_origin);
        if let Some(origin) = body_origin {
            char_root.body_origin = origin;
        }
    }
}

pub fn on_use_skill(
    trigger: On<UseSkill>,
    mut commands: Commands,
    skill_db: Res<SkillDatabase>,
    mut char_query: Query<(&mut CharacterAnimation, &Transform, &CharacterRoot), With<CharacterRoot>>,
) {
    let ev = trigger.event();
    let Some(skill) = skill_db.get(ev.skill_id) else {
        warn!("skill {} not found in database", ev.skill_id);
        return;
    };
    let Ok((mut anim, char_transform, char_root)) = char_query.get_mut(ev.entity) else {
        return;
    };
    info!(
        "on_use_skill: id={} name={} char_pos=({:.1},{:.1})",
        ev.skill_id, skill.name, char_transform.translation.x, char_transform.translation.y,
    );
    if anim.return_to_default {
        anim.pending_action = Some(PendingCharacterAction::Skill {
            skill_id: ev.skill_id,
        });
        return;
    }

    if skill.effect_frames.is_empty() {
        return;
    }

    // Play character animation if the skill has one
    if let Some(action) = &skill.action {
        commands.trigger(SetAction {
            entity: ev.entity,
            action: action.clone(),
            return_to_default: true,
        });
    }

    // Spawn effect frames as child of character root
    let effect_root = commands
        .spawn((
            Name::new("SkillEffect"),
            SkillEffectRoot,
            Transform::from_translation(Vec3::new(0.0, 0.0, GameLayer::Skill.with_offset(0.0))),
            Visibility::Visible,
        ))
        .id();

    commands.entity(ev.entity).add_child(effect_root);

    info!(
        "skill effect root: child of char root at local ZERO (body origin/pivot), char at ({:.1},{:.1})",
        char_transform.translation.x, char_transform.translation.y,
    );

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
                Transform::from_xyz(-first.origin.x, -first.origin.y, first.z as f32),
                Visibility::Visible,
            ))
            .id();
        commands.entity(effect_root).add_child(sprite_entity);
    }

    // Skill name label below the character
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
            Transform::from_translation(Vec3::new(
                0.0,
                -60.0,
                GameLayer::Skill.with_offset(0.0),
            )),
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
            for child in children.iter() {
                commands.entity(child).despawn();
            }
            commands.entity(entity).despawn();
            continue;
        }

        let frame = &effect.frames[effect.frame_idx];
        let image = frame.image.clone();
        let origin = frame.origin;
        let z = frame.z;
        let delay = frame.delay;
        effect.timer = Timer::from_seconds(delay as f32 / 1000.0, TimerMode::Repeating);

        if let Some(child) = children.iter().next() {
            if let Ok((mut sprite, mut child_transform)) = sprite_query.get_mut(child) {
                sprite.image = image;
                child_transform.translation = Vec3::new(-origin.x, -origin.y, z as f32);
            }
        }
    }
}

pub fn on_character_action(
    trigger: On<crate::input::ActionEvent>,
    mut query: Query<
        (
            Entity,
            &mut Job,
            &mut CharacterConfig,
            Option<&Children>,
            &mut CharacterAnimation,
        ),
        (With<CharacterRoot>, With<IsLocalPlayer>),
    >,
    mut cycle: ResMut<CharacterActionCycle>,
    skill_db: Res<SkillDatabase>,
    job_catalog: Res<JobCatalog>,
    mut job_label_query: Query<&mut Text2d, With<CharacterJobLabel>>,
    mut commands: Commands,
) {
    let (entity, mut job, mut config, children, mut current_anim) = match query.iter_mut().next() {
        Some(result) => result,
        None => return,
    };

    let selected_action = match trigger.event().0 {
        KeyAction::Jump | KeyAction::JumpAction => "jump",
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
        KeyAction::CycleJob => {
            let Some(next_job) = job_catalog.next_after(*job) else {
                return;
            };
            *job = next_job;
            config.job = next_job;
            let label = job_catalog.display_label(next_job);
            if let Some(children) = children {
                for child in children.iter() {
                    if let Ok(mut job_label) = job_label_query.get_mut(child) {
                        job_label.0.clone_from(&label);
                    }
                }
            }
            return;
        }
        KeyAction::CycleSkill => {
            let skill_db = skill_db.as_ref();
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
            return;
        }
        _ => return,
    };
    let return_to_default = selected_action != DEFAULT_CHARACTER_ACTION;
    if current_anim.return_to_default {
        current_anim.pending_action = Some(PendingCharacterAction::Action {
            action: selected_action.to_string(),
            return_to_default,
        });
    } else {
        commands.trigger(SetAction {
            entity,
            action: selected_action.to_string(),
            return_to_default,
        });
    }
}

pub fn animate_characters(
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &mut CharacterAnimation,
        &CharacterFrameData,
        &PartEntities,
        &mut CharacterRoot,
    )>,
    mut part_query: Query<(&mut Sprite, &mut Transform, &CharacterPart)>,
    mut commands: Commands,
) {
    for (entity, mut anim, frame_data, part_entities, mut char_root) in &mut query {
        let mut animation_dirty = false;

        anim.timer.tick(time.delta());
        if anim.timer.just_finished() {
            if let Some(frames) = frame_data.actions.get(&anim.action) {
                if !frames.is_empty() {
                    let next_frame_idx = anim.frame_idx + 1;
                    if anim.return_to_default && next_frame_idx >= frames.len() {
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
                        continue;
                    }

                    anim.frame_idx = next_frame_idx % frames.len();
                    let frame = &frames[anim.frame_idx];
                    anim.timer =
                        Timer::from_seconds(frame.delay as f32 / 1000.0, TimerMode::Repeating);
                    animation_dirty = true;
                }
            }
        }

        anim.face_timer.tick(time.delta());
        if anim.face_timer.just_finished() {
            if let Some(face_frames) = frame_data.face_expressions.get(&anim.face_expression) {
                if !face_frames.is_empty() {
                    let next_idx = (anim.face_frame_idx + 1) % face_frames.len();
                    anim.face_frame_idx = next_idx;
                    if next_idx == 0 {
                        anim.face_timer =
                            Timer::from_seconds(1.0, TimerMode::Repeating);
                    } else {
                        let face_frame = &face_frames[anim.face_frame_idx];
                        let fd = face_frame.delay.max(50);
                        anim.face_timer =
                            Timer::from_seconds(fd as f32 / 1000.0, TimerMode::Repeating);
                    }
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

        let (positions, _) = compute_frame_transforms(&merged_parts, GameLayer::Character.base_z());

        char_root.body_origin = merged_parts
            .iter()
            .find(|p| p.layer_name == "body")
            .map(|body| body.origin)
            .unwrap_or(char_root.body_origin);

        for layer in &merged_parts {
            if let Some(&child) = part_entities.map.get(&layer.layer_name) {
                let pos = positions
                    .get(&layer.layer_name)
                    .copied()
                    .unwrap_or(Vec3::new(0.0, 0.0, GameLayer::Character.base_z() + layer.z));
                update_part_sprite(
                    child,
                    layer,
                    pos,
                    anim.facing_left,
                    &mut part_query,
                );
            } else {
                debug!(
                    "animate: part '{}' not found in part_entities (frame_idx={}, action='{}', source={:?})",
                    layer.layer_name, anim.frame_idx, anim.action, layer.source,
                );
            }
        }
    }
}

pub fn on_set_action(
    trigger: On<SetAction>,
    mut query: Query<(
        &mut CharacterAnimation,
        &CharacterFrameData,
        &PartEntities,
        &Children,
        &mut CharacterRoot,
    )>,
    mut part_query: Query<(&mut Sprite, &mut Transform, &CharacterPart)>,
    mut label_query: Query<&mut Text2d, With<CharacterActionLabel>>,
) {
    let ev = trigger.event();
    let Ok((mut anim, frame_data, part_entities, children, mut char_root)) = query.get_mut(ev.entity) else {
        return;
    };

    let Some(frames) = frame_data.actions.get(&ev.action) else {
        warn!("action '{}' not found for character", ev.action);
        return;
    };
    if frames.is_empty() {
        warn!("action '{}' has no frames", ev.action);
        return;
    }

    if anim.return_to_default {
        anim.pending_action = Some(PendingCharacterAction::Action {
            action: ev.action.clone(),
            return_to_default: ev.return_to_default,
        });
        return;
    }

    anim.action = ev.action.clone();
    anim.return_to_default = ev.return_to_default;
    anim.frame_idx = 0;
    for child in children.iter() {
        if let Ok(mut label) = label_query.get_mut(child) {
            label.0.clone_from(&ev.action);
        }
    }

    if let Some(first) = frames.first() {
        anim.timer = Timer::from_seconds(first.delay as f32 / 1000.0, TimerMode::Repeating);

        let mut merged_parts = first.parts.clone();
        if let Some(face_frames) = frame_data.face_expressions.get(&anim.face_expression) {
            if let Some(face_frame) =
                face_frames.get(anim.face_frame_idx.min(face_frames.len().saturating_sub(1)))
            {
                merged_parts.extend(face_frame.parts.clone());
            }
        }

        let (positions, _) = compute_frame_transforms(&merged_parts, GameLayer::Character.base_z());

        char_root.body_origin = merged_parts
            .iter()
            .find(|p| p.layer_name == "body")
            .map(|body| body.origin)
            .unwrap_or(char_root.body_origin);

        for layer in &merged_parts {
            if let Some(&child) = part_entities.map.get(&layer.layer_name) {
                let pos = positions
                    .get(&layer.layer_name)
                    .copied()
                    .unwrap_or(Vec3::ZERO);
                update_part_sprite(
                    child,
                    layer,
                    pos,
                    anim.facing_left,
                    &mut part_query,
                );
            } else {
                debug!(
                    "on_set_action: part '{}' not found in part_entities (action='{}', source={:?})",
                    layer.layer_name, ev.action, layer.source,
                );
            }
        }
    }
}
