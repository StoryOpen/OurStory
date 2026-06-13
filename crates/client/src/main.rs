mod camera;
#[cfg(feature = "character")]
mod character;
mod input;
mod layer;
mod physics;
mod wz;

#[cfg(feature = "map")]
mod map;
#[cfg(feature = "mob")]
mod mob;
#[cfg(feature = "npc")]
mod npc;
#[cfg(feature = "ui")]
mod ui;

const WORLD_X: DiagnosticPath = DiagnosticPath::const_new("world/x");
const WORLD_Y: DiagnosticPath = DiagnosticPath::const_new("world/y");
const SCREEN_X: DiagnosticPath = DiagnosticPath::const_new("screen/x");
const SCREEN_Y: DiagnosticPath = DiagnosticPath::const_new("screen/y");

#[derive(Resource, Default)]
pub struct SelectedEntity(pub Option<Entity>);

use bevy::camera::ScalingMode;
use bevy::dev_tools::diagnostics_overlay::{
    DiagnosticsOverlay, DiagnosticsOverlayItem, DiagnosticsOverlayPlugin,
    DiagnosticsOverlayStatistic,
};
use bevy::diagnostic::{
    Diagnostic, DiagnosticPath, Diagnostics, FrameTimeDiagnosticsPlugin, RegisterDiagnostic,
};
use bevy::prelude::*;
use camera::CameraPlugin;
#[cfg(feature = "character")]
use character::CharacterPlugin;
use clap::Parser;
use input::InputPlugin;
use wz::asset_source::WzAssetSourcePlugin;
#[derive(Parser)]
struct Args {
    #[arg(long)]
    map: Option<String>,
}

#[cfg(feature = "map")]
use map::MapPlugin;
#[cfg(feature = "mob")]
use mob::MobPlugin;
#[cfg(feature = "npc")]
use npc::NpcPlugin;
#[cfg(feature = "ui")]
use ui::UiPlugin;
#[cfg(feature = "inspector")]
use bevy_inspector_egui::quick::WorldInspectorPlugin;
#[cfg(feature = "inspector")]
use bevy_inspector_egui::bevy_egui::{EguiPlugin, EguiContext, PrimaryEguiContext, EguiPrimaryContextPass};
#[cfg(feature = "inspector")]
use bevy_inspector_egui::bevy_inspector::ui_for_entity;
#[cfg(feature = "inspector")]
use bevy_inspector_egui::egui;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSet {
    Input,
    Physics,
    Animation,
    Camera,
    Audio,
    Ui,
    Visuals,
}

fn main() {
    let args = Args::parse();
    let workspace_id: String = std::env::var("WORKSPACE_ID").unwrap_or_default();

    let title = if workspace_id.is_empty() {
        "OurStory".to_string()
    } else {
        format!("OurStory{workspace_id}")
    };

    let mut app = App::new();
    app.add_plugins(WzAssetSourcePlugin)
        .add_plugins(bevy::remote::RemotePlugin::default())
        .add_plugins(bevy::remote::http::RemoteHttpPlugin::default())
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_linear())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title,
                        resolution: bevy::window::WindowResolution::new(
                            camera::resources::BaseResolution::default().width as u32,
                            camera::resources::BaseResolution::default().height as u32,
                        ),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(DiagnosticsOverlayPlugin)
        .register_diagnostic(
            Diagnostic::new(WORLD_X)
                .with_suffix("px")
                .with_max_history_length(1)
                .with_smoothing_factor(0.0),
        )
        .register_diagnostic(
            Diagnostic::new(WORLD_Y)
                .with_suffix("px")
                .with_max_history_length(1)
                .with_smoothing_factor(0.0),
        )
        .register_diagnostic(
            Diagnostic::new(SCREEN_X)
                .with_suffix("px")
                .with_max_history_length(1)
                .with_smoothing_factor(0.0),
        )
        .register_diagnostic(
            Diagnostic::new(SCREEN_Y)
                .with_suffix("px")
                .with_max_history_length(1)
                .with_smoothing_factor(0.0),
        )
        .configure_sets(
            Update,
            (
                GameSet::Input,
                physics::PhysicsSet::Simulate,
                GameSet::Animation,
                GameSet::Camera,
                GameSet::Audio,
                GameSet::Ui,
                GameSet::Visuals,
            )
                .chain(),
        );

    #[cfg(feature = "character")]
    app.add_plugins(CharacterPlugin);
    app.add_plugins(CameraPlugin)
        .add_plugins(InputPlugin)
        .add_plugins(physics::PhysicsPlugin);
    #[cfg(feature = "map")]
    app.add_plugins(MapPlugin { start_map: args.map.or_else(|| Some("Map/Map/Map1/100010000.img".into())), ..default() });
    #[cfg(feature = "mob")]
    app.add_plugins(MobPlugin::default());
    #[cfg(feature = "npc")]
    app.add_plugins(NpcPlugin::default());
    #[cfg(feature = "ui")]
    app.add_plugins(UiPlugin);
    #[cfg(feature = "inspector")]
    app.add_plugins(EguiPlugin::default());
    #[cfg(feature = "inspector")]
    app.add_plugins(WorldInspectorPlugin::new());
    #[cfg(feature = "inspector")]
    app.add_systems(EguiPrimaryContextPass, selected_entity_ui);

    app.add_observer(wz::set_sprite_bottom_left)
        .insert_resource(SelectedEntity(None))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (select_entity_on_click, write_coords, draw_entity_gizmos, draw_selected_entity_gizmo).in_set(GameSet::Visuals),
        )
        .run();
}

fn setup(mut commands: Commands) {
    let viewport_height = camera::resources::BaseResolution::default().height;
    commands.spawn((
        Name::new("MainCamera"),
        Camera2d,
        camera::resources::MainCamera,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical { viewport_height },
            ..OrthographicProjection::default_2d()
        })
    ));
    commands.insert_resource(physics::load_physics());
    commands.spawn((
        Name::new("DiagnosticsOverlay"),
        DiagnosticsOverlay {
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
    }));
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

fn select_entity_on_click(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
    sprites: Query<(Entity, &Transform, &Sprite)>,
    images: Res<Assets<Image>>,
    mut selected: ResMut<SelectedEntity>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some((camera, camera_transform)) = camera.iter().next() else { return };
    let Some(window) = window.iter().next() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor) else { return };
    let cursor_world = ray.origin.truncate();

    let mut best_entity = None;
    let mut best_z = f32::NEG_INFINITY;

    for (entity, transform, sprite) in &sprites {
        let size = sprite
            .custom_size
            .or_else(|| images.get(&sprite.image).map(|i| i.size_f32()))
            .unwrap_or(Vec2::splat(32.0));
        let min = transform.translation.truncate();
        let max = min + size;
        if cursor_world.x >= min.x && cursor_world.x <= max.x
            && cursor_world.y >= min.y && cursor_world.y <= max.y
            && transform.translation.z > best_z
        {
            best_z = transform.translation.z;
            best_entity = Some(entity);
        }
    }

    selected.0 = best_entity;
}

fn draw_selected_entity_gizmo(
    selected: Res<SelectedEntity>,
    query: Query<(&GlobalTransform, &Sprite)>,
    images: Res<Assets<Image>>,
    mut gizmos: Gizmos,
) {
    let Some(entity) = selected.0 else { return };
    let Ok((transform, sprite)) = query.get(entity) else { return };
    let size = sprite
        .custom_size
        .or_else(|| images.get(&sprite.image).map(|i| i.size_f32()))
        .unwrap_or(Vec2::splat(32.0));
    let pos = transform.translation().truncate() + size * 0.5;
    gizmos.rect_2d(pos, size, Color::srgba(1.0, 0.3, 0.3, 0.7));
}

#[cfg(feature = "inspector")]
fn selected_entity_ui(world: &mut World) {
    let entity = world.resource::<SelectedEntity>().0;
    let Some(entity) = entity else { return };
    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryEguiContext>>()
        .single(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();
    egui::Window::new("Selected Entity")
        .default_size((320.0, 200.0))
        .show(egui_context.get_mut(), |ui| {
            ui_for_entity(world, entity, ui);
        });
}

fn draw_entity_gizmos(
    mut gizmos: Gizmos,
    query: Query<(&GlobalTransform, &Sprite)>,
    images: Res<Assets<Image>>,
) {
    for (transform, sprite) in &query {
        let size = sprite
            .custom_size
            .or_else(|| images.get(&sprite.image).map(|i| i.size_f32()))
            .unwrap_or(Vec2::splat(32.0));
        let pos = transform.translation().truncate() + size * 0.5;
        gizmos.rect_2d(pos, size, Color::srgba(0.0, 1.0, 0.0, 0.3));
    }
}
