mod character;
mod map;
mod mob;
mod wz;

const WORLD_X: DiagnosticPath = DiagnosticPath::const_new("world/x");
const WORLD_Y: DiagnosticPath = DiagnosticPath::const_new("world/y");
const SCREEN_X: DiagnosticPath = DiagnosticPath::const_new("screen/x");
const SCREEN_Y: DiagnosticPath = DiagnosticPath::const_new("screen/y");

use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
};
use bevy::diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, FrameTimeDiagnosticsPlugin, RegisterDiagnostic};
use bevy::dev_tools::diagnostics_overlay::{
    DiagnosticsOverlay, DiagnosticsOverlayItem, DiagnosticsOverlayPlugin, DiagnosticsOverlayStatistic,
};
use wz::asset_source::WzAssetSourcePlugin;

use character::{CharacterPlugin, components::CharacterConfig, events::SpawnCharacter, types::EquipSlot};

fn main() {
    App::new()
        .add_plugins(WzAssetSourcePlugin)
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_linear()))
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(DiagnosticsOverlayPlugin)
        .register_diagnostic(Diagnostic::new(WORLD_X).with_suffix("px").with_max_history_length(1).with_smoothing_factor(0.0))
        .register_diagnostic(Diagnostic::new(WORLD_Y).with_suffix("px").with_max_history_length(1).with_smoothing_factor(0.0))
        .register_diagnostic(Diagnostic::new(SCREEN_X).with_suffix("px").with_max_history_length(1).with_smoothing_factor(0.0))
        .register_diagnostic(Diagnostic::new(SCREEN_Y).with_suffix("px").with_max_history_length(1).with_smoothing_factor(0.0))
        .add_plugins(map::MapPlugin::default())
        .add_plugins(CharacterPlugin)
        .add_plugins(mob::MobPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Startup, draw_grid)
        .add_systems(Update, drag_camera)
        .add_systems(Update, write_coords)
        .add_systems(Update, debug_cycle_actions)
        .run();
}

fn draw_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    window: Query<&Window>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let win = match window.iter().next() {
        Some(w) => w,
        None => return,
    };
    let height = win.height();
    let width = win.width();

    let short_v = meshes.add(Rectangle::new(5.0, 100.0));
    let short_h = meshes.add(Rectangle::new(100.0, 5.0));
    let long_v = meshes.add(Rectangle::new(1.0, height));
    let long_h = meshes.add(Rectangle::new(width, 1.0));

    commands.spawn((
        Mesh2d(short_h),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Srgba::RED))),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    commands.spawn((
        Mesh2d(short_v),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Srgba::RED))),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    let h2 = height as i32 / 2 / 100;
    for i in -h2..h2 + 1 {
        let y = i as f32 * 100.0;
        commands.spawn((
            Mesh2d(long_h.clone()),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(Srgba::WHITE))),
            Transform::from_xyz(0.0, y, 0.0),
        ));
    }
    let w2 = width as i32 / 2 / 100;
    for i in -w2..w2 + 1 {
        let x = i as f32 * 100.0;
        commands.spawn((
            Mesh2d(long_v.clone()),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(Srgba::WHITE))),
            Transform::from_xyz(x, 0.0, 0.0),
        ));
    }
}

fn debug_cycle_actions(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    let actions = ["stand", "move", "hit1", "die1"];
    for (i, action) in actions.iter().enumerate() {
        let key = match i {
            0 => KeyCode::Digit1,
            1 => KeyCode::Digit2,
            2 => KeyCode::Digit3,
            3 => KeyCode::Digit4,
            _ => continue,
        };
        if keys.just_pressed(key) {
            commands.trigger(mob::events::SwitchMobAction {
                mob_id: 100100,
                action: action.to_string(),
            });
            bevy::log::info!("switch Snail to {action}");
        }
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.trigger(SpawnCharacter {
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        config: CharacterConfig {
            skin_suffix: 2000,
            hair_id: 31200,
            face_id: 21405,
            equipment: vec![
                (EquipSlot::Cap, 01002419),
                (EquipSlot::Coat, 01042013),
                (EquipSlot::Pants, 01060135),
                (EquipSlot::Shoes, 01072306),
                (EquipSlot::Glove, 01082178),
                (EquipSlot::Weapon, 01452000),
                (EquipSlot::Shield, 01092027),
                (EquipSlot::Cape, 01102149),
            ],
        },
        action: "stand1".into(),
        face_expression: "default".into(),
    });
    commands.trigger(mob::events::SpawnMob { mob_id: 100100, x: 0.0, y: 0.0, z: 100 });
    commands.spawn(DiagnosticsOverlay {
        title: "Debug".into(),
        diagnostic_overlay_items: vec![
            DiagnosticsOverlayItem {
                path: WORLD_X,
                statistic: DiagnosticsOverlayStatistic::Value,
                precision: 1,
            },
            DiagnosticsOverlayItem {
                path: WORLD_Y,
                statistic: DiagnosticsOverlayStatistic::Value,
                precision: 1,
            },
            DiagnosticsOverlayItem {
                path: SCREEN_X,
                statistic: DiagnosticsOverlayStatistic::Value,
                precision: 1,
            },
            DiagnosticsOverlayItem {
                path: SCREEN_Y,
                statistic: DiagnosticsOverlayStatistic::Value,
                precision: 1,
            },
            FrameTimeDiagnosticsPlugin::FPS.into(),
            FrameTimeDiagnosticsPlugin::FRAME_TIME.into(),
            DiagnosticsOverlayItem {
                path: FrameTimeDiagnosticsPlugin::FRAME_COUNT,
                statistic: DiagnosticsOverlayStatistic::Smoothed,
                precision: 0,
            },
        ],
    });
}

fn drag_camera(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    mut camera: Query<&mut Transform, With<Camera>>,
) {
    if accumulated_mouse_motion.delta == Vec2::ZERO || !mouse_button_input.pressed(MouseButton::Left) {
        return;
    }
    let mut camera_transform = match camera.iter_mut().next() {
        Some(t) => t,
        None => return,
    };
    camera_transform.translation += (accumulated_mouse_motion.delta * Vec2::new(-1.0, 1.0)).extend(0.0);
}

fn write_coords(
    window: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
    mut diagnostics: Diagnostics,
) {
    let Some((camera, camera_transform)) = camera.iter().next() else {
        return;
    };
    let Some(window) = window.iter().next() else {
        return;
    };

    if let Some(world_position) = window
        .cursor_position()
        .map(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .and_then(|ray| ray.ok())
        .map(|ray| ray.origin.trunc())
    {
        diagnostics.add_measurement(&WORLD_X, || world_position.x as f64);
        diagnostics.add_measurement(&WORLD_Y, || world_position.y as f64);
    }
    if let Some(cursor_position) = window.cursor_position() {
        diagnostics.add_measurement(&SCREEN_X, || cursor_position.x as f64);
        diagnostics.add_measurement(&SCREEN_Y, || cursor_position.y as f64);
    }
}
