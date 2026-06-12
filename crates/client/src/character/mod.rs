pub mod components;
pub mod events;
pub mod job;
pub mod loader;
pub mod skill_loader;
pub mod skills;
pub mod systems;
pub mod types;

use bevy::prelude::*;

use self::loader::WzSpriteCache;
use self::skills::SkillDatabase;
use self::systems::*;
use self::types::{load_smap, load_zmap};
use crate::GameSet;
use crate::map::events::MapLoaded;
use crate::wz::get_cached_base;

pub struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        let base = get_cached_base();
        let mut cache = WzSpriteCache::default();
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        let skill_db = skill_loader::load_skill_database(base, &mut cache, &mut images);
        app.insert_resource(load_zmap(base))
            .insert_resource(load_smap(base))
            .insert_resource(job::load_job_catalog(base))
            .insert_resource(cache)
            .insert_resource(skill_db)
            .init_resource::<CharacterActionCycle>()
            .register_type::<components::CharacterRoot>()
            .register_type::<components::CharacterPart>()
            .register_type::<components::CharacterLayer>()
            .add_observer(spawn_character)
            .add_observer(on_set_action)
            .add_observer(on_set_facing)
            .add_observer(on_character_action)
            .add_observer(on_use_skill)
            .add_observer(spawn_character_on_map)
            .add_systems(
                Update,
                update_character_facing_from_intent
                    .in_set(GameSet::Input)
                    .after(crate::input::dispatch_actions),
            )
            .add_systems(Update, animate_characters.in_set(GameSet::Animation))
            .add_systems(Update, animate_skill_effects.in_set(GameSet::Animation));
    }
}

fn spawn_character_on_map(
    trigger: On<MapLoaded>,
    mut commands: Commands,
    assets: Res<Assets<crate::map::asset_loader::WzMapAsset>>,
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
        .portals
        .iter()
        .find(|p| p.pt == 0)
        .map(|p| p.pos)
        .unwrap_or(Vec2::ZERO);

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
                (EquipSlot::Cap, 01000000),
                (EquipSlot::Cape, 01102000),
                (EquipSlot::Coat, 01040000),
                (EquipSlot::Pants, 01060000),
                (EquipSlot::Shoes, 01070000),
                (EquipSlot::Glove, 01080000),
                (EquipSlot::Weapon, 01302000),
                (EquipSlot::Shield, 01092000),
                (EquipSlot::Accessory, 01010000),
                (EquipSlot::Ring, 01112000),
            ],
        },
        action: DEFAULT_CHARACTER_ACTION.into(),
        face_expression: "blink".into(),
    });
}
