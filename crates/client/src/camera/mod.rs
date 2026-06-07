pub mod resources;
pub mod systems;

use crate::GameSet;
use bevy::prelude::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<resources::BaseResolution>()
            .add_systems(Startup, systems::apply_resolution)
            .add_systems(
                Update,
                (
                    systems::follow_player,
                    systems::drag_camera,
                    // TEMP: clamp_camera disabled — clamp_camera.after(systems::drag_camera),
                )
                    .in_set(GameSet::Camera),
            );
    }
}
