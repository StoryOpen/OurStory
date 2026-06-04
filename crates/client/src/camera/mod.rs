pub mod resources;
pub mod systems;

use bevy::prelude::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_observer(systems::reset_camera)
            .add_systems(Startup, systems::apply_resolution)
            .add_systems(Update, (
                systems::follow_player.after(crate::physics::PhysicsSet::Simulate),
                systems::drag_camera,
                systems::clamp_camera.after(systems::drag_camera),
            ));
    }
}
