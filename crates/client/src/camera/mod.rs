pub mod resources;
pub mod systems;

use bevy::prelude::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<resources::BaseResolution>()
            .add_systems(Startup, systems::apply_resolution)
            .add_systems(Update, (
                systems::follow_player.after(crate::physics::PhysicsSet::Simulate),
                systems::drag_camera,
                // TEMP: clamp_camera disabled — clamp_camera.after(systems::drag_camera),
            ));
    }
}
