use std::collections::HashMap;
use bevy::prelude::*;

use crate::physics::{PhysicsSet, PhysicsState};

/// Marker for the local player's character entity.
#[derive(Component)]
pub struct IsLocalPlayer;

/// What a character *wants* to do — source agnostic.
/// Written by keyboard input, network packets, or AI.
/// Applied to `PhysicsState` by `apply_intent` before physics simulation.
#[derive(Component, Default)]
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
    Stand1,
    Walk1,
    Sit,
    Prone,
    Ladder,
    Rope,
    Fly,
    Alert,
    Dead,
    SwingO1,
    SwingP1,
    Shoot1,
    Magic1,
    // Developer debug
    DebugMobStand,
    DebugMobMove,
    DebugMobHit1,
    DebugMobDie1,
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
        self.inner.iter()
            .filter(|&(_, a)| *a == action)
            .map(|(&k, _)| k)
            .collect()
    }
}

impl Default for KeyBindings {
    fn default() -> Self {
        let mut inner = HashMap::new();
        inner.insert(KeyCode::ArrowLeft,  KeyAction::MoveLeft);
        inner.insert(KeyCode::ArrowRight, KeyAction::MoveRight);
        inner.insert(KeyCode::ArrowUp,    KeyAction::MoveUp);
        inner.insert(KeyCode::ArrowDown,  KeyAction::MoveDown);
        inner.insert(KeyCode::Space,      KeyAction::Jump);
        inner.insert(KeyCode::Digit1,     KeyAction::Stand1);
        inner.insert(KeyCode::Digit2,     KeyAction::Walk1);
        inner.insert(KeyCode::Digit3,     KeyAction::JumpAction);
        inner.insert(KeyCode::Digit4,     KeyAction::Sit);
        inner.insert(KeyCode::Digit5,     KeyAction::Prone);
        inner.insert(KeyCode::Digit6,     KeyAction::Ladder);
        inner.insert(KeyCode::Digit7,     KeyAction::Rope);
        inner.insert(KeyCode::Digit8,     KeyAction::Fly);
        inner.insert(KeyCode::Digit9,     KeyAction::Alert);
        inner.insert(KeyCode::Digit0,     KeyAction::Dead);
        inner.insert(KeyCode::KeyQ,       KeyAction::SwingO1);
        inner.insert(KeyCode::KeyW,       KeyAction::SwingP1);
        inner.insert(KeyCode::KeyE,       KeyAction::Shoot1);
        inner.insert(KeyCode::KeyR,       KeyAction::Magic1);
        inner.insert(KeyCode::F5,         KeyAction::DebugMobStand);
        inner.insert(KeyCode::F6,         KeyAction::DebugMobMove);
        inner.insert(KeyCode::F7,         KeyAction::DebugMobHit1);
        inner.insert(KeyCode::F8,         KeyAction::DebugMobDie1);
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
    let mut intent = match local_player.iter_mut().next() {
        Some(i) => i,
        None => return,
    };

    for (&key, &action) in &bindings.inner {
        match action {
            KeyAction::MoveLeft  => intent.left = keyboard.pressed(key),
            KeyAction::MoveRight => intent.right = keyboard.pressed(key),
            KeyAction::MoveUp    => intent.up = keyboard.pressed(key),
            KeyAction::MoveDown  => intent.down = keyboard.pressed(key),
            KeyAction::Jump => {
                if keyboard.just_pressed(key) {
                    intent.jump_request = true;
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
pub fn apply_intent(
    mut query: Query<(&CharacterIntent, &mut PhysicsState)>,
) {
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
           .add_systems(Update, (
               dispatch_actions,
               apply_intent.before(PhysicsSet::Simulate),
           ));
    }
}
