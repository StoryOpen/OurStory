use bevy::prelude::*;

use crate::character::components::*;
use crate::character::events::*;
use crate::character::types::EquipSlot;
use crate::character::weapon_stances::COMMON_STANCE_VARIANTS;
use crate::input::IsLocalPlayer;
use crate::physics::PhysicsState;

use super::DEFAULT_CHARACTER_ACTION;

const ALERT_DURATION_SECS: f32 = 2.0;

// ── Component ──

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct CharacterStance {
    pub stance: Stance,
    pub sub_stance: IdleSubStance,
    pub alert_timer: Timer,
    combat_animation_done: bool,
}

impl Default for CharacterStance {
    fn default() -> Self {
        Self {
            stance: Stance::Idle,
            sub_stance: IdleSubStance::Stand,
            alert_timer: Timer::from_seconds(ALERT_DURATION_SECS, TimerMode::Once),
            combat_animation_done: false,
        }
    }
}

// ── Enums ──

#[derive(PartialEq, Clone, Copy, Debug, Reflect)]
pub enum Stance {
    Combat,
    Alert,
    Idle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum IdleSubStance {
    Stand,
    Walk,
    Rope,
    Ladder,
    Sit,
    Prone,
    Fly,
    Jump,
}

// ── Events ──

#[derive(EntityEvent)]
pub struct RequestAttack {
    pub entity: Entity,
    pub action: String,
}

#[derive(EntityEvent)]
pub struct HitByMob {
    pub entity: Entity,
}

// ── Observers ──

pub fn on_request_attack(
    trigger: On<RequestAttack>,
    mut commands: Commands,
    mut query: Query<(&mut CharacterStance, &mut CharacterActionAnimation), With<CharacterRoot>>,
) {
    let ev = trigger.event();
    let Ok((mut stance, _anim)) = query.get_mut(ev.entity) else {
        return;
    };

    if stance.stance == Stance::Combat {
        return;
    }

    stance.stance = Stance::Combat;
    stance.combat_animation_done = false;

    commands.trigger(SetAction {
        entity: ev.entity,
        action: ev.action.clone(),
        return_to_default: false,
    });
}

pub fn on_hit_by_mob(
    trigger: On<HitByMob>,
    mut commands: Commands,
    mut query: Query<(&mut CharacterStance, Entity), With<CharacterRoot>>,
) {
    let ev = trigger.event();
    let Ok((mut stance, entity)) = query.get_mut(ev.entity) else {
        return;
    };

    if stance.stance != Stance::Idle {
        return;
    }

    stance.stance = Stance::Alert;
    stance.alert_timer.reset();

    commands.trigger(SetAction {
        entity,
        action: "alert".to_string(),
        return_to_default: false,
    });
}

// ── Systems ──

pub fn tick_stance(
    time: Res<Time>,
    mut query: Query<
        (Entity, &mut CharacterStance, &CharacterActionAnimation, &PhysicsState, &CharacterConfig),
        With<CharacterRoot>,
    >,
    mut commands: Commands,
) {
    for (entity, mut stance, anim, phys, config) in &mut query {
        match stance.stance {
            Stance::Combat => {
                if stance.combat_animation_done {
                    continue;
                }
                let total_frames = if anim.frame_count > 1 {
                    2 * (anim.frame_count - 1)
                } else {
                    1
                };
                if anim.frame_count > 0 && anim.frame_idx + 1 >= total_frames {
                    stance.combat_animation_done = true;
                    stance.stance = Stance::Alert;
                    stance.alert_timer.reset();
                    commands.trigger(SetAction {
                        entity,
                        action: "alert".to_string(),
                        return_to_default: false,
                    });
                }
            }
            Stance::Alert => {
                stance.alert_timer.tick(time.delta());
                if !stance.alert_timer.just_finished() {
                    continue;
                }

                stance.stance = Stance::Idle;
                let sub = determine_idle_sub_stance(phys);
                stance.sub_stance = sub;

                let action = idle_action_for_sub_stance(sub, config);
                commands.trigger(SetAction {
                    entity,
                    action,
                    return_to_default: false,
                });
            }
            Stance::Idle => {}
        }
    }
}

pub fn update_movement_stance(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &PhysicsState,
            &mut CharacterStance,
            &CharacterActionAnimation,
            &CharacterConfig,
        ),
        (With<CharacterRoot>, With<IsLocalPlayer>),
    >,
) {
    for (entity, phys, mut stance, anim, config) in &mut query {
        if stance.stance != Stance::Idle {
            continue;
        }

        let new_sub = determine_idle_sub_stance(phys);
        if new_sub != stance.sub_stance {
            stance.sub_stance = new_sub;
            let action = idle_action_for_sub_stance(new_sub, config);
            commands.trigger(SetAction {
                entity,
                action,
                return_to_default: false,
            });
        }

        let moving = phys.left || phys.right;
        if moving {
            let facing_left = phys.left;
            if anim.facing_left != facing_left {
                commands.trigger(SetFacing {
                    entity,
                    facing_left,
                });
            }
        }
    }
}

// ── Helpers ──

fn determine_idle_sub_stance(phys: &PhysicsState) -> IdleSubStance {
    let moving = phys.left || phys.right;
    if moving {
        return IdleSubStance::Walk;
    }
    IdleSubStance::Stand
}

fn weapon_type_code(config: &CharacterConfig) -> Option<&'static str> {
    let item_id = config
        .equipment
        .iter()
        .find(|(slot, _)| *slot == EquipSlot::Weapon)?
        .1;
    let code = format!("{:04}", item_id / 10000);
    Some(Box::leak(code.into_boxed_str()))
}

fn idle_action_for_sub_stance(sub: IdleSubStance, config: &CharacterConfig) -> String {
    match sub {
        IdleSubStance::Stand => {
            if let Some(code) = weapon_type_code(config) {
                if let Some(variants) = COMMON_STANCE_VARIANTS
                    .iter()
                    .find(|(k, _)| *k == code)
                    .map(|(_, v)| *v)
                {
                    if let Some(stand) = variants.iter().find(|v| v.starts_with("stand")) {
                        return stand.to_string();
                    }
                }
            }
            DEFAULT_CHARACTER_ACTION.to_string()
        }
        IdleSubStance::Walk => {
            if let Some(code) = weapon_type_code(config) {
                if let Some(variants) = COMMON_STANCE_VARIANTS
                    .iter()
                    .find(|(k, _)| *k == code)
                    .map(|(_, v)| *v)
                {
                    if let Some(walk) = variants.iter().find(|v| v.starts_with("walk")) {
                        return walk.to_string();
                    }
                }
            }
            "walk1".to_string()
        }
        IdleSubStance::Rope => "rope".to_string(),
        IdleSubStance::Ladder => "ladder".to_string(),
        IdleSubStance::Sit => "sit".to_string(),
        IdleSubStance::Prone => "prone".to_string(),
        IdleSubStance::Fly => "fly".to_string(),
        IdleSubStance::Jump => "jump".to_string(),
    }
}
