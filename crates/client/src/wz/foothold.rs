pub use wz::Foothold;

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
