use bevy::prelude::*;
use std::collections::HashSet;

use crate::character::components::*;
use crate::character::events::*;
use crate::character::job::{Job, JobCatalog};
use crate::character::skills::SkillDatabase;
use crate::character::stance::RequestAttack;
use crate::input::{IsLocalPlayer, KeyAction};

use super::DEFAULT_CHARACTER_ACTION;

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
            &mut CharacterLabels,
            &mut CharacterActionAnimation,
        ),
        (With<CharacterRoot>, With<IsLocalPlayer>),
    >,
    mut cycle: ResMut<ActionCycle>,
    action_lists: Res<ActionLists>,
    skill_db: Res<SkillDatabase>,
    job_catalog: Res<JobCatalog>,
    mut commands: Commands,
) {
    let (entity, mut job, mut config, mut labels, mut current_anim) = match query.iter_mut().next() {
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
        KeyAction::Attack => {
            commands.trigger(RequestAttack {
                entity,
                action: "swingO1".to_string(),
            });
        }
        KeyAction::CycleJob => {
            let Some(next_job) = job_catalog.next_after(*job) else {
                return;
            };
            *job = next_job;
            config.job = next_job;
            let label = job_catalog.display_label(next_job);
            labels.job.clone_from(&label);
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
