#[cfg(feature = "character")]
mod character;
mod camera;
mod input;
mod physics;
mod wz;

#[cfg(feature = "map")]
mod map;
#[cfg(feature = "mob")]
mod mob;
#[cfg(feature = "ui")]
mod ui;

const WORLD_X: DiagnosticPath = DiagnosticPath::const_new("world/x");
const WORLD_Y: DiagnosticPath = DiagnosticPath::const_new("world/y");
const SCREEN_X: DiagnosticPath = DiagnosticPath::const_new("screen/x");
const SCREEN_Y: DiagnosticPath = DiagnosticPath::const_new("screen/y");

use bevy::prelude::*;
use bevy::camera::ScalingMode;
use bevy::diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, FrameTimeDiagnosticsPlugin, RegisterDiagnostic};
use bevy::dev_tools::diagnostics_overlay::{
    DiagnosticsOverlay, DiagnosticsOverlayItem, DiagnosticsOverlayPlugin, DiagnosticsOverlayStatistic,
};
#[cfg(feature = "character")]
use character::CharacterPlugin;
use camera::CameraPlugin;
use input::InputPlugin;
use wz::asset_source::WzAssetSourcePlugin;
use wz::get_cached_base;

#[cfg(feature = "map")]
use map::MapPlugin;
#[cfg(feature = "mob")]
use mob::MobPlugin;
#[cfg(feature = "ui")]
use ui::UiPlugin;

fn main() {
    let workspace_id: String = std::env::var("WORKSPACE_ID").unwrap_or_default();

    let title = if workspace_id.is_empty() {
        "OurStory".to_string()
    } else {
        format!("OurStory{workspace_id}")
    };

    let mut app = App::new();
    app.add_plugins(WzAssetSourcePlugin)
       .add_plugins(bevy::remote::RemotePlugin::default())
       .add_plugins(bevy::remote::RemoteHttpPlugin::default())
       .add_plugins(DefaultPlugins.set(ImagePlugin::default_linear()).set(WindowPlugin {
           primary_window: Some(Window { title, ..default() }),
           ..default()
       }))
       .add_plugins(FrameTimeDiagnosticsPlugin::default())
       .add_plugins(DiagnosticsOverlayPlugin)
       .register_diagnostic(Diagnostic::new(WORLD_X).with_suffix("px").with_max_history_length(1).with_smoothing_factor(0.0))
       .register_diagnostic(Diagnostic::new(WORLD_Y).with_suffix("px").with_max_history_length(1).with_smoothing_factor(0.0))
       .register_diagnostic(Diagnostic::new(SCREEN_X).with_suffix("px").with_max_history_length(1).with_smoothing_factor(0.0))
       .register_diagnostic(Diagnostic::new(SCREEN_Y).with_suffix("px").with_max_history_length(1).with_smoothing_factor(0.0));

    #[cfg(feature = "character")]
    app.add_plugins(CharacterPlugin);
    app.add_plugins(CameraPlugin)
       .add_plugins(InputPlugin)
       .add_plugins(physics::PhysicsPlugin);
    #[cfg(feature = "map")]
    app.add_plugins(MapPlugin::default());
    #[cfg(feature = "mob")]
    app.add_plugins(MobPlugin::default());
    #[cfg(feature = "ui")]
    app.add_plugins(UiPlugin);

    app.add_systems(Startup, (setup, draw_grid))
       .add_systems(Update, write_coords)
       .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        camera::resources::MainCamera,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical { viewport_height: 768.0 },
            ..OrthographicProjection::default_2d()
        }),
    ));
    commands.insert_resource(camera::resources::BaseResolution { width: 1024.0, height: 768.0 });
    commands.insert_resource(physics::load_physics(get_cached_base()));
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
