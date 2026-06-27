#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Vector2D(pub f32, pub f32);

impl Vector2D {
    pub const ZERO: Vector2D = Vector2D(0.0, 0.0);
}
