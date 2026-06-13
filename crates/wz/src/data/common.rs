use crate::vector2d::Vector2D;

#[derive(Debug, Clone)]
pub struct AnimFrame {
    pub image_path: String,
    pub origin: Vector2D,
    pub delay: u32,
    pub move_type: i32,
    pub move_w: f32,
    pub move_h: f32,
    pub move_p: f32,
    pub move_r: f32,
    pub a0: f32,
    pub a1: f32,
}

#[derive(Debug, Clone)]
pub struct FrameData {
    pub parts: Vec<SpriteLayerData>,
    pub delay: u32,
}

#[derive(Debug, Clone)]
pub struct SpriteLayerData {
    pub image_path: String,
    pub origin: Vector2D,
    pub map: std::collections::HashMap<String, Vector2D>,
    pub z: f32,
    pub layer_name: String,
    pub slot: Option<String>,
    pub source: PartSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartSource {
    Body,
    Head,
    Hair,
    Face,
    Equipment,
}

#[derive(Debug, Clone)]
pub struct Foothold {
    pub id: i32,
    pub group: i32,
    pub layer: u8,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub force: Option<i32>,
    pub forbid_fall: Option<i32>,
    pub piece: Option<i32>,
    pub next_id: Option<i32>,
    pub prev_id: Option<i32>,
    pub cant_through: bool,
    pub forbid_fall_down: bool,
}

impl Foothold {
    pub fn y_at(&self, x: f32) -> f32 {
        if (self.x2 - self.x1).abs() < f32::EPSILON {
            self.y1
        } else {
            let t = ((x - self.x1) / (self.x2 - self.x1)).clamp(0.0, 1.0);
            self.y1 + t * (self.y2 - self.y1)
        }
    }

    pub fn contains_x(&self, x: f32) -> bool {
        let lo = self.x1.min(self.x2);
        let hi = self.x1.max(self.x2);
        x >= lo && x <= hi
    }
}
