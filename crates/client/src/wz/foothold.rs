#[derive(Debug, Clone, Copy)]
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

pub fn layer_at(footholds: &[Foothold], x: f32, y: f32) -> Option<u8> {
    footholds
        .iter()
        .filter(|f| f.contains_x(x))
        .filter(|f| (f.y_at(x) - y).abs() < 300.0 && f.y_at(x) >= y - 50.0)
        .min_by(|a, b| {
            let da = (a.y_at(x) - y).abs();
            let db = (b.y_at(x) - y).abs();
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|f| f.layer)
}
