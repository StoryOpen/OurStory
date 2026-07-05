use bevy::prelude::*;

/// Tracks whether critical startup assets have been loaded.
/// Systems that need physics constants, zmap, or other WZ data
/// should check this resource before proceeding.
#[derive(Resource, Default)]
pub struct LoadingState {
    pub physics_loaded: bool,
    pub zmap_loaded: bool,
    pub hud_sprites_loaded: bool,
    /// Set true when all critical resources are ready
    pub ready: bool,
}

