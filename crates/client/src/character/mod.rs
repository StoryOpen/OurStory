pub mod components;
pub mod events;
pub mod job;
pub mod skills;
pub mod systems;
pub mod types;

use bevy::prelude::*;

use self::skills::SkillDatabase;
use self::systems::*;
use self::types::{load_zmap, WzDataRes};
use crate::GameSet;
#[cfg(feature = "map")]
use crate::map::events::MapLoaded;

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        let wz = wz::WzData::global();
        let skill_db = SkillDatabase::load(wz);

        app.insert_resource(WzDataRes(wz))
            .insert_resource(load_zmap(wz))
            .insert_resource(job::load_job_catalog(wz))
            .insert_resource(skill_db)
            .insert_resource(load_action_lists(wz))
            .init_resource::<ActionCycle>()
            .register_type::<components::CharacterActionAnimation>()
            .register_type::<components::CharacterConfig>()
            .register_type::<components::CharacterLabels>()
            .register_type::<components::CharacterFaceAnimation>()
            .register_type::<components::CharacterActionLabel>()
            .register_type::<components::CharacterJobLabel>()
            .register_type::<components::SkillNameLabel>()
            .register_type::<types::ZMap>()
            .register_type::<types::EquipSlot>()
            .register_type::<job::Job>()
            .register_type::<job::JobCatalog>()
            .register_type::<skills::SkillDatabase>()
            .register_type::<skills::LearnedSkills>()
            .register_type::<skills::SkillEffect>()
            .register_type::<skills::SkillEffectRoot>()
            .register_type::<ActionLists>()
            .register_type::<ActionCycle>()
            .add_observer(spawn_character)
            .add_observer(on_set_action)
            .add_observer(on_set_face_expression)
            .add_observer(on_set_facing)
            .add_observer(on_character_action)
            .add_observer(on_use_skill);
        #[cfg(feature = "map")]
        app.add_observer(spawn_character_on_map);
        app.add_systems(
                Update,
                update_character_facing_from_intent
                    .in_set(GameSet::Input)
                    .after(crate::input::dispatch_actions),
            )
            .add_systems(Update, advance_character_frames.in_set(GameSet::Animation))
            .add_systems(Update, animate_face.in_set(GameSet::Animation))
            .add_systems(Update, animate_skill_effects.in_set(GameSet::Animation));

        #[cfg(not(feature = "map"))]
        app.add_systems(Startup, spawn_character_at_origin);
    }
}

#[cfg(feature = "map")]
fn spawn_character_on_map(
    trigger: On<MapLoaded>,
    mut commands: Commands,
    assets: Res<Assets<crate::wz::asset_loaders::WzMapAsset>>,
) {
    let ev = trigger.event();
    info!("MapLoaded: {}", ev.path);
    let asset = match assets.get(&ev.handle) {
        Some(a) => a,
        None => {
            warn!("Map asset not found for {}", ev.path);
            return;
        }
    };

    let spawn_pos = asset
        .0
        .portals
        .iter()
        .find(|p| p.pt == 0)
        .map(|p| Vec2::new(p.pos.0, p.pos.1))
        .unwrap_or_else(|| {
            warn!("spawn_character_on_map: map '{}' has no spawn portal (pt=0), using ZERO", ev.path);
            Vec2::ZERO
        });

    info!("Spawning character at spawn portal: {:?}", spawn_pos);

    use crate::character::{
        components::CharacterConfig, events::SpawnCharacter, job::Job, types::EquipSlot,
    };
    commands.trigger(SpawnCharacter {
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        config: CharacterConfig {
            skin_suffix: 2000,
            hair_id: 30000,
            face_id: 20000,
            job: Job(112),
            equipment: vec![
                (EquipSlot::Cap, 01002000),
                (EquipSlot::Cape, 01102000),
                (EquipSlot::Coat, 01040002),
                (EquipSlot::Pants, 01060001),
                (EquipSlot::Shoes, 01072000),
                (EquipSlot::Glove, 01080000),
                (EquipSlot::Weapon, 01302000),
                (EquipSlot::Shield, 01092003),
                (EquipSlot::Accessory, 01010000),
                (EquipSlot::Ring, 01112000),
            ],
        },
        action: DEFAULT_CHARACTER_ACTION.into(),
        face_expression: "blink".into(),
    });
}

#[cfg(not(feature = "map"))]
fn spawn_character_at_origin(mut commands: Commands) {
    use crate::character::{
        components::CharacterConfig, events::SpawnCharacter, job::Job, types::EquipSlot,
    };
    commands.trigger(SpawnCharacter {
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        config: CharacterConfig {
            skin_suffix: 2000,
            hair_id: 30000,
            face_id: 20000,
            job: Job(112),
            equipment: vec![],
        },
        action: DEFAULT_CHARACTER_ACTION.into(),
        face_expression: "blink".into(),
    });
}
