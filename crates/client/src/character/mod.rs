pub mod components;
pub mod events;
pub mod job;
pub mod skills;
pub mod stance;
pub mod systems;
pub mod types;
pub mod weapon_stances;

use bevy::prelude::*;

use self::job::JobCatalog;
use self::skills::SkillDatabase;
use self::systems::*;
use self::types::zmap_from_entries;
use crate::GameSet;
use crate::ui::loading::LoadingState;
use crate::wz::asset_loaders::*;
use crate::map::events::MapLoaded;

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<components::CharacterActionAnimation>()
            .register_type::<components::CharacterConfig>()
            .register_type::<components::CharacterLabels>()
            .register_type::<components::CharacterFaceAnimation>()
            .register_type::<types::ZMap>()
            .register_type::<types::EquipSlot>()
            .register_type::<job::Job>()
            .register_type::<job::JobCatalog>()
            .register_type::<skills::SkillDatabase>()
            .register_type::<skills::LearnedSkills>()
            .register_type::<skills::SkillEffect>()
            .register_type::<skills::SkillEffectRoot>()
            .register_type::<systems::input::ActionLists>()
            .register_type::<systems::input::ActionCycle>()
            .add_observer(spawn_character)
            .add_observer(on_set_action)
            .add_observer(on_set_face_expression)
            .add_observer(on_set_facing)
            .add_observer(on_character_action)
            .add_observer(on_use_skill)
            .add_observer(stance::on_request_attack)
            .add_observer(stance::on_hit_by_mob)
            .add_systems(Startup, init_startup_assets);
        app.add_observer(spawn_character_on_map);
        app.add_systems(
                Update,
                stance::update_movement_stance
                    .in_set(GameSet::Input)
                    .after(crate::input::dispatch_actions),
            )
            .add_systems(
                Update,
                stance::tick_stance
                    .in_set(GameSet::Animation)
                    .after(advance_character_frames),
            )
            .add_systems(Update, advance_character_frames.in_set(GameSet::Animation))
            .add_systems(Update, animate_face.in_set(GameSet::Animation))
            .add_systems(Update, animate_skill_effects.in_set(GameSet::Animation))
            .add_systems(Update, process_pending_action_load.in_set(GameSet::Animation))
            .add_systems(Update, process_pending_face_load.in_set(GameSet::Animation))
            .add_systems(Update, draw_character_labels.in_set(GameSet::Visuals));
    }
}

/// Startup system that loads singleton character assets (zmap, skill db, job catalog, action lists).
/// Runs once when all assets are ready.
pub fn init_startup_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    zmap_assets: Res<Assets<WzZMapAsset>>,
    skill_assets: Res<Assets<WzSkillDatabaseAsset>>,
    job_assets: Res<Assets<WzJobCatalogAsset>>,
    act_assets: Res<Assets<WzActionListsAsset>>,
    mut loading_state: Option<ResMut<LoadingState>>,
    mut initialized: Local<bool>,
) {
    if *initialized {
        return;
    }

    let zmap_handle = asset_server.load::<WzZMapAsset>("wz://zmap.zmap");
    let skill_handle = asset_server.load::<WzSkillDatabaseAsset>("wz://skill-database.skilldb");
    let job_handle = asset_server.load::<WzJobCatalogAsset>("wz://job-catalog.jobcat");
    let act_handle = asset_server.load::<WzActionListsAsset>("wz://action-lists.actlist");

    let zmap_ready = zmap_assets.get(&zmap_handle);
    let skill_ready = skill_assets.get(&skill_handle);
    let job_ready = job_assets.get(&job_handle);
    let act_ready = act_assets.get(&act_handle);

    if let (Some(zmap), Some(skill), Some(job), Some(act)) =
        (zmap_ready, skill_ready, job_ready, act_ready)
    {
        commands.insert_resource(zmap_from_entries(zmap.0.clone()));
        commands.insert_resource(SkillDatabase::from_raw(&skill.0));
        commands.insert_resource(JobCatalog::from_raw(&job.0));
        commands.insert_resource(systems::input::ActionLists::from_raw(&act));
        if let Some(ref mut loading) = loading_state {
            loading.zmap_loaded = true;
            loading.ready = loading.physics_loaded && loading.zmap_loaded;
        }
        *initialized = true;
        info!("Character startup assets loaded");
    }
}

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
        .data
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
