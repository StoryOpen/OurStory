#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Vector2D {
    pub x: f32,
    pub y: f32,
}

impl Vector2D {
    pub const ZERO: Vector2D = Vector2D { x: 0.0, y: 0.0 };
}

impl crate::node_trait::TryFromNode<crate::node::Node> for Vector2D {
    fn try_from_node(node: crate::node::Node) -> Result<Self, crate::error::WzError> {
        let v = wz_reader::property::Vector2D::try_from(node)?;
        Ok(Vector2D { x: v.0 as f32, y: v.1 as f32 })
    }
}
