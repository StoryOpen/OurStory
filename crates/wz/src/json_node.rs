use image::DynamicImage;
use indexmap::IndexMap;
use serde_json::Value;
use std::collections::HashMap;
use std::hash::Hash;
use crate::error::WzError;
use crate::vector2d::Vector2D;
use crate::NodePayload;

/// A JSON-backed node that mirrors the `Node` API for WASM usage.
/// Constructed from a `NodePayload` fetched via HTTP.
#[derive(Clone, Debug)]
pub struct JsonNode {
    pub name: String,
    pub path: String,
    pub value: Option<Value>,
    pub children: IndexMap<String, JsonNode>,
}

impl JsonNode {
    pub fn from_payload(payload: &NodePayload) -> Self {
        Self::from_payload_with_parent(payload, "")
    }

    fn from_payload_with_parent(payload: &NodePayload, parent_path: &str) -> Self {
        let path = if parent_path.is_empty() {
            payload.name.clone()
        } else {
            format!("{}/{}", parent_path, payload.name)
        };
        let mut children = IndexMap::new();
        for child in &payload.children {
            children.insert(
                child.name.clone(),
                JsonNode::from_payload_with_parent(child, &path),
            );
        }
        JsonNode {
            name: payload.name.clone(),
            path,
            value: payload.value.clone(),
            children,
        }
    }

    pub fn at_path(&self, path: &str) -> Result<JsonNode, WzError> {
        if path.is_empty() {
            return Err(WzError::NodeNotFound(path.to_string()));
        }
        let segments: Vec<&str> = path.split('/').collect();
        let mut current = self;
        for segment in &segments {
            current = current.children.get(*segment)
                .ok_or_else(|| WzError::NodeNotFound(format!("{}/{}", current.name, segment)))?;
        }
        Ok(current.clone())
    }

    pub fn children(&self) -> IndexMap<String, JsonNode> {
        self.children.clone()
    }

    pub fn try_get(&self, name: &str) -> Option<JsonNode> {
        self.children.get(name).cloned()
    }

    pub fn has(&self, name: &str) -> bool {
        self.children.contains_key(name)
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn read_pos(&self) -> Result<Vector2D, WzError> {
        let x: f32 = self.at_path("x")?.try_into()?;
        let y: f32 = self.at_path("y")?.try_into()?;
        Ok(Vector2D(x, -(y as f32)))
    }

    pub fn read_pos_n(&self, n: u8) -> Result<Vector2D, WzError> {
        let x: f32 = self.at_path(&format!("x{n}"))?.try_into()?;
        let y: f32 = self.at_path(&format!("y{n}"))?.try_into()?;
        Ok(Vector2D(x as f32, -(y as f32)))
    }

    pub fn get_or<T: TryFrom<JsonNode, Error = WzError>>(&self, path: &str, default: T) -> T {
        self.at_path(path)
            .ok()
            .and_then(|n| T::try_from(n).ok())
            .unwrap_or(default)
    }

    pub fn get_opt<T: TryFrom<JsonNode, Error = WzError>>(&self, path: &str) -> Option<T> {
        self.at_path(path).ok().and_then(|n| T::try_from(n).ok())
    }

    pub fn required<T: TryFrom<JsonNode, Error = WzError>>(&self, path: &str) -> T {
        self.at_path(path)
            .unwrap_or_else(|_| panic!("required child '{path}' not found in JSON node '{}'", self.name))
            .try_into()
            .unwrap_or_else(|_| panic!("required child '{path}' type mismatch in JSON node '{}'", self.name))
    }

    pub fn ordered_children(&self) -> Result<Vec<(String, JsonNode)>, WzError> {
        Ok(self.children.clone().into_iter().collect())
    }

    pub fn has_image_data(&self) -> bool {
        // In WASM mode, image data is fetched separately via HTTP.
        self.value.as_ref().and_then(|v| v.get("type")).and_then(|t| t.as_str()) == Some("PNG")
            || self.has("_inlink")
            || self.has("_outlink")
    }

    pub fn path(&self) -> String {
        self.path.clone()
    }

    pub fn read_origin(&self, _img_node: &JsonNode) -> Result<Vector2D, WzError> {
        let origin = self.at_path("origin")?;
        let x: f32 = origin.at_path("x")?.try_into().unwrap_or(0.0);
        let y: f32 = origin.at_path("y")?.try_into().unwrap_or(0.0);
        Ok(Vector2D(x, y))
    }

    pub fn extract_image(&self) -> Result<DynamicImage, WzError> {
        Err(WzError::ValueError("extract_image not available from JsonNode".into()))
    }
}

// ═════════════════════════════════════════════════════════════
//  WzNode trait implementation (shared interface with Node)
// ═════════════════════════════════════════════════════════════

use crate::node_trait::{WzNode, TryFromNode};

impl WzNode for JsonNode {
    fn at_path(&self, path: &str) -> Result<Self, WzError> { JsonNode::at_path(self, path) }
    fn children(&self) -> IndexMap<String, Self> { JsonNode::children(self) }
    fn try_get(&self, name: &str) -> Option<Self> { JsonNode::try_get(self, name) }
    fn has(&self, name: &str) -> bool { JsonNode::has(self, name) }
    fn name(&self) -> String { JsonNode::name(self) }
    fn path(&self) -> String { JsonNode::path(self) }
    fn read_pos(&self) -> Result<Vector2D, WzError> { JsonNode::read_pos(self) }
    fn read_pos_n(&self, n: u8) -> Result<Vector2D, WzError> { JsonNode::read_pos_n(self, n) }
    fn read_origin(&self, img_node: &Self) -> Result<Vector2D, WzError> { JsonNode::read_origin(self, img_node) }
    fn ordered_children(&self) -> Result<Vec<(String, Self)>, WzError> { JsonNode::ordered_children(self) }
    fn has_image_data(&self) -> bool { JsonNode::has_image_data(self) }
    fn extract_image(&self) -> Result<DynamicImage, WzError> { JsonNode::extract_image(self) }
}

impl TryFromNode<JsonNode> for i32 {
    fn try_from_node(node: JsonNode) -> Result<Self, WzError> { i32::try_from(node) }
}
impl TryFromNode<JsonNode> for f32 {
    fn try_from_node(node: JsonNode) -> Result<Self, WzError> { f32::try_from(node) }
}
impl TryFromNode<JsonNode> for String {
    fn try_from_node(node: JsonNode) -> Result<Self, WzError> { String::try_from(node) }
}
impl TryFromNode<JsonNode> for bool {
    fn try_from_node(node: JsonNode) -> Result<Self, WzError> { bool::try_from(node) }
}
impl TryFromNode<JsonNode> for wz_reader::property::Vector2D {
    fn try_from_node(node: JsonNode) -> Result<Self, WzError> {
        let x: i32 = node.get_opt("x").unwrap_or(0);
        let y: i32 = node.get_opt("y").unwrap_or(0);
        Ok(wz_reader::property::Vector2D(x, y))
    }
}



impl TryFrom<JsonNode> for i32 {
    type Error = WzError;
    fn try_from(node: JsonNode) -> Result<Self, Self::Error> {
        if let Some(v) = &node.value {
            if let Some(n) = v.as_i64() {
                return Ok(n as i32);
            }
            if let Some(s) = v.as_str() {
                if let Ok(n) = s.parse::<i32>() {
                    return Ok(n);
                }
            }
            // Try "Int" or "Short" tagged value
            if let Some(data) = v.get("data") {
                if let Some(n) = data.as_i64() {
                    return Ok(n as i32);
                }
            }
        }
        Err(WzError::TypeMismatch("i32"))
    }
}

impl TryFrom<JsonNode> for f32 {
    type Error = WzError;
    fn try_from(node: JsonNode) -> Result<Self, Self::Error> {
        if let Some(v) = &node.value {
            if let Some(n) = v.as_f64() {
                return Ok(n as f32);
            }
            if let Some(data) = v.get("data") {
                if let Some(n) = data.as_f64() {
                    return Ok(n as f32);
                }
            }
        }
        // Fallback to i32
        let n: i32 = node.try_into()?;
        Ok(n as f32)
    }
}

impl TryFrom<JsonNode> for String {
    type Error = WzError;
    fn try_from(node: JsonNode) -> Result<Self, Self::Error> {
        if let Some(v) = &node.value {
            if let Some(s) = v.as_str() {
                return Ok(s.to_string());
            }
            if let Some(data) = v.get("data") {
                if let Some(s) = data.as_str() {
                    return Ok(s.to_string());
                }
            }
        }
        Err(WzError::TypeMismatch("String"))
    }
}

impl TryFrom<JsonNode> for bool {
    type Error = WzError;
    fn try_from(node: JsonNode) -> Result<Self, Self::Error> {
        let value: i32 = node.try_into()?;
        Ok(value != 0)
    }
}

impl<T: TryFrom<JsonNode, Error = WzError>> TryFrom<JsonNode> for Vec<T> {
    type Error = WzError;
    fn try_from(value: JsonNode) -> Result<Self, Self::Error> {
        Ok(value
            .children()
            .into_iter()
            .filter(|(key, _)| key.parse::<u32>().is_ok())
            .filter_map(|(_, node)| node.try_into().ok())
            .collect())
    }
}

impl<T: TryFrom<JsonNode, Error = WzError>, K: TryFrom<String> + Hash + Eq> TryFrom<JsonNode> for HashMap<K, T> {
    type Error = WzError;
    fn try_from(value: JsonNode) -> Result<Self, Self::Error> {
        Ok(value
            .children()
            .into_iter()
            .filter_map(|(key, node)| Some((K::try_from(key).ok()?, node.try_into().ok()?)))
            .collect())
    }
}

impl<T: TryFrom<JsonNode, Error = WzError>, K: TryFrom<String> + Hash + Eq> TryFrom<JsonNode> for IndexMap<K, T> {
    type Error = WzError;
    fn try_from(value: JsonNode) -> Result<Self, Self::Error> {
        Ok(value
            .children()
            .into_iter()
            .filter_map(|(key, node)| Some((K::try_from(key).ok()?, node.try_into().ok()?)))
            .collect())
    }
}
