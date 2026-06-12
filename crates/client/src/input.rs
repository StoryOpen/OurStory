use bevy::prelude::*;
use std::collections::HashMap;

use crate::GameSet;
use crate::physics::PhysicsState;

/// Marker for the local player's character entity.
#[derive(Component, Reflect)]
pub struct IsLocalPlayer;

/// What a character *wants* to do — source agnostic.
/// Written by keyboard input, network packets, or AI.
/// Applied to `PhysicsState` by `apply_intent` before physics simulation.
#[derive(Component, Default, Reflect)]
pub struct CharacterIntent {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub jump_request: bool,
}

/// Every discrete action a key can be bound to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyAction {
    // Movement (continuous state)
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    // Discrete triggers
    Jump,
    JumpAction,
    // Category cycling
    CycleStance,
    CycleAlert,
    CycleSwing,
    CycleStab,
    CycleMultiSwing,
    CycleRanged,
    CycleMagic,
    CycleMovementSkill,
    CycleSkill,
    CycleJob,
}

/// Live key → action mapping. Players can swap bindings at runtime.
#[derive(Resource)]
pub struct KeyBindings {
    inner: HashMap<KeyCode, KeyAction>,
}

impl KeyBindings {
    pub fn set(&mut self, key: KeyCode, action: KeyAction) {
        self.inner.insert(key, action);
    }

    pub fn get(&self, key: KeyCode) -> Option<&KeyAction> {
        self.inner.get(&key)
    }

    pub fn keys_for(&self, action: KeyAction) -> Vec<KeyCode> {
        self.inner
            .iter()
            .filter(|&(_, a)| *a == action)
            .map(|(&k, _)| k)
            .collect()
    }
}

impl Default for KeyBindings {
    fn default() -> Self {
        let mut inner = HashMap::new();
        inner.insert(KeyCode::ArrowLeft, KeyAction::MoveLeft);
        inner.insert(KeyCode::ArrowRight, KeyAction::MoveRight);
        inner.insert(KeyCode::ArrowUp, KeyAction::MoveUp);
        inner.insert(KeyCode::ArrowDown, KeyAction::MoveDown);
        // Jump animation
        inner.insert(KeyCode::KeyQ, KeyAction::JumpAction);
        // Category cycling keys
        inner.insert(KeyCode::KeyA, KeyAction::CycleStance);
        inner.insert(KeyCode::KeyS, KeyAction::CycleSwing);
        inner.insert(KeyCode::KeyD, KeyAction::CycleStab);
        inner.insert(KeyCode::KeyF, KeyAction::CycleMultiSwing);
        inner.insert(KeyCode::KeyZ, KeyAction::CycleRanged);
        inner.insert(KeyCode::KeyX, KeyAction::CycleMagic);
        inner.insert(KeyCode::KeyC, KeyAction::CycleMovementSkill);
        inner.insert(KeyCode::KeyV, KeyAction::CycleSkill);
        inner.insert(KeyCode::KeyJ, KeyAction::CycleJob);
        inner.insert(KeyCode::KeyB, KeyAction::CycleAlert);
        Self { inner }
    }
}

/// Fired once per press for every discrete action bound to a key.
/// All listeners are registered via `.add_observer` in their respective plugins.
#[derive(Event)]
pub struct ActionEvent(pub KeyAction);

/// Single choke point for all keyboard input.
///
/// 1. Movement bindings → written to `CharacterIntent` (continuous `pressed` state).
/// 2. `Jump` → sets `jump_request` on intent + emits `ActionEvent`.
/// 3. Everything else → emits `ActionEvent` on `just_pressed`.
///
/// Consumer systems listen for `ActionEvent` and never read `ButtonInput<KeyCode>` directly.
pub fn dispatch_actions(
    bindings: Res<KeyBindings>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut local_player: Query<&mut CharacterIntent, With<IsLocalPlayer>>,
    mut commands: Commands,
) {
    let mut intent = local_player.iter_mut().next();

    for (&key, &action) in &bindings.inner {
        match action {
            KeyAction::MoveLeft
            | KeyAction::MoveRight
            | KeyAction::MoveUp
            | KeyAction::MoveDown => {
                if let Some(ref mut i) = intent {
                    match action {
                        KeyAction::MoveLeft => i.left = keyboard.pressed(key),
                        KeyAction::MoveRight => i.right = keyboard.pressed(key),
                        KeyAction::MoveUp => i.up = keyboard.pressed(key),
                        KeyAction::MoveDown => i.down = keyboard.pressed(key),
                        _ => {}
                    }
                }
            }
            KeyAction::Jump => {
                if keyboard.just_pressed(key) {
                    if let Some(ref mut i) = intent {
                        i.jump_request = true;
                    }
                    commands.trigger(ActionEvent(action));
                }
            }
            _ => {
                if keyboard.just_pressed(key) {
                    commands.trigger(ActionEvent(action));
                }
            }
        }
    }
}

/// Copies `CharacterIntent` into `PhysicsState` for every entity that has both.
/// Runs before `PhysicsSet::Simulate` so the physics step consumes the latest intent.
pub fn apply_intent(mut query: Query<(&CharacterIntent, &mut PhysicsState)>) {
    for (intent, mut ps) in &mut query {
        ps.left = intent.left;
        ps.right = intent.right;
        ps.up = intent.up;
        ps.down = intent.down;
        ps.jump_request = intent.jump_request;
    }
}

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KeyBindings>()
            .register_type::<IsLocalPlayer>()
            .add_systems(
                Update,
                (dispatch_actions, apply_intent).in_set(GameSet::Input),
            );
    }
}
