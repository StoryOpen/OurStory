/// Game-semantic rendering layers.
///
/// Each layer has a base z. Plugins use `base_z()` + their own within-layer
/// offset, eliminating cross-plugin z coordination.
///
/// Lower z = rendered behind. All layers fit within Bevy's default 2D
/// orthographic clip range [-1000, 1000].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameLayer {
    /// Background scenery (sky, mountains, far objects).
    Background,
    /// Map objects that render behind characters.
    ObjBehind,
    /// Map floor tiles and platforms.
    Tile,
    /// Monsters.
    Mob,
    /// Players and NPCs.
    Character,
    /// Map objects that render in front of characters.
    ObjFront,
    /// Foreground overlays (front backgrounds, weather effects).
    Foreground,
}

impl GameLayer {
    /// Base z for this layer.
    pub fn base_z(self) -> f32 {
        match self {
            Self::Background => -900.0,
            Self::ObjBehind => -600.0,
            Self::Tile => -200.0,
            Self::Mob => 200.0,
            Self::Character => 400.0,
            Self::ObjFront => 600.0,
            Self::Foreground => 800.0,
        }
    }

    /// Returns `base_z + offset`, for within-layer ordering.
    pub fn with_offset(self, offset: f32) -> f32 {
        self.base_z() + offset
    }
}
