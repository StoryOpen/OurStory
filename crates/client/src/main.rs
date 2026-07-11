mod camera;
mod input;
mod layer;
mod physics;
mod wz;

mod map;
mod ui;

use bevy::camera::ScalingMode;
use bevy::prelude::*;
use camera::CameraPlugin;
use input::InputPlugin;
use wz::asset_source::WzAssetSourcePlugin;

use map::MapPlugin;
use ui::UiPlugin;

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

fn build_app(title: &str) -> App {
    let mut app = App::new();
    app.add_plugins(WzAssetSourcePlugin)
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_linear())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: title.into(),
                        resolution: bevy::window::WindowResolution::new(
                            camera::resources::BaseResolution::default().width as u32,
                            camera::resources::BaseResolution::default().height as u32,
                        ),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(wz::WzAssetPlugin)
        .configure_sets(
            Update,
            (
                GameSet::Input,
                GameSet::Animation,
                GameSet::Camera,
                GameSet::Audio,
                GameSet::Ui,
                GameSet::Visuals,
            )
                .chain(),
        );

    app.add_plugins(CameraPlugin)
        .add_plugins(InputPlugin)
        .add_plugins(physics::PhysicsPlugin);
    app.add_plugins(MapPlugin::default());
    app.add_plugins(UiPlugin);

    app.add_observer(wz::set_sprite_bottom_left);

    app
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // Determine API base URL from page context.
    // The API path is derived from the <base href="..."> tag:
    //   - base href = "/"     → API at "/wz/..."  (prod)
    //   - base href = "/dev/" → API at "/dev-wz/..." (dev)
    let api_base_url = web_sys::window()
        .and_then(|w| {
            let origin = w.location().origin().ok()?;
            let doc = w.document()?;
            // Check for <base href> tag (used in dev deployment to set API path)
            let base_href = doc
                .query_selector("base")
                .ok()?
                .and_then(|el| el.get_attribute("href"));
            let api_path = match base_href.as_deref() {
                // Root deployment: API is at /wz/... (same origin)
                Some("/") | None => String::new(),
                // Non-root deployment: e.g. /dev/ → /dev-wz
                Some(path) => {
                    let trimmed = path.trim_end_matches('/');
                    format!("{}-wz", trimmed)
                }
            };
            Some(format!("{}{}", origin, api_path))
        })
        // Fallback: use page origin (handles dev server on non-standard ports)
        .unwrap_or_else(|| {
            web_sys::window()
                .and_then(|w| w.location().origin().ok())
                .unwrap_or_default()
        });

    // Install panic hook for better error messages on wasm
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    // Initialize WzData with HTTP API backend
    // Note: ::wz refers to the external wz crate (local mod wz shadows it)
    ::wz::WzData::init_wasm(api_base_url);

    build_app("OurStory").add_systems(Startup, setup).run();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let workspace_id: String = std::env::var("WORKSPACE_ID").unwrap_or_default();

    let title = if workspace_id.is_empty() {
        "OurStory".to_string()
    } else {
        format!("OurStory{workspace_id}")
    };

    build_app(&title)
        .add_plugins(bevy::remote::RemotePlugin::default())
        .add_plugins(bevy::remote::http::RemoteHttpPlugin::default())
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.insert_resource(ClearColor(Color::WHITE));
    let viewport_height = camera::resources::BaseResolution::default().height;
    commands.spawn((
        Name::new("MainCamera"),
        Camera2d,
        camera::resources::MainCamera,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical { viewport_height },
            ..OrthographicProjection::default_2d()
        }),
    ));
}
