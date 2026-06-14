use bevy::prelude::*;
use std::collections::HashMap;

use crate::GameSet;
use crate::physics::PhysicsState;

/// Marker for the local player's character entity.
#[derive(Component, Reflect)]
pub struct IsLocalPlayer;

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
    // Action cycling
    CycleBasic,
    CycleComposite,
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
        inner.insert(KeyCode::KeyQ, KeyAction::CycleBasic);
        // Category cycling keys
        inner.insert(KeyCode::KeyW, KeyAction::CycleComposite);
        inner.insert(KeyCode::KeyJ, KeyAction::CycleJob);
        inner.insert(KeyCode::KeyV, KeyAction::CycleSkill);
        inner.insert(KeyCode::Space, KeyAction::JumpAction);
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
    mut local_player: Query<&mut PhysicsState, With<IsLocalPlayer>>,
    mut commands: Commands,
) {
    let mut phys = local_player.iter_mut().next();

    for (&key, &action) in &bindings.inner {
        match action {
            KeyAction::MoveLeft
            | KeyAction::MoveRight
            | KeyAction::MoveUp
            | KeyAction::MoveDown => {
                if let Some(ref mut p) = phys {
                    match action {
                        KeyAction::MoveLeft => p.left = keyboard.pressed(key),
                        KeyAction::MoveRight => p.right = keyboard.pressed(key),
                        KeyAction::MoveUp => p.up = keyboard.pressed(key),
                        KeyAction::MoveDown => p.down = keyboard.pressed(key),
                        _ => {}
                    }
                }
            }
            KeyAction::Jump => {
                if keyboard.just_pressed(key) {
                    if let Some(ref mut p) = phys {
                        p.jump_request = true;
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

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KeyBindings>()
            .register_type::<IsLocalPlayer>()
            .add_systems(
                Update,
                dispatch_actions.in_set(GameSet::Input),
            );
    }
}
