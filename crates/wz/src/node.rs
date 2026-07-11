use image::DynamicImage;
use indexmap::IndexMap;
use log::warn;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::hash::Hash;
use wz_reader::{WzNodeArc, WzNodeCast, WzNodeName};

use crate::error::WzError;
use crate::vector2d::Vector2D;

#[derive(Clone)]
pub struct Node {
    pub wz_node: WzNodeArc,
}

impl From<WzNodeArc> for Node {
    fn from(val: WzNodeArc) -> Self {
        Node { wz_node: val }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NodeName {
    pub wz_name: WzNodeName,
}

impl From<WzNodeName> for NodeName {
    fn from(val: WzNodeName) -> Self {
        NodeName { wz_name: val }
    }
}

impl NodeName {
    pub fn to_string(&self) -> String {
        self.wz_name.to_string()
    }
    pub fn as_str(&self) -> &str {
        self.wz_name.as_str()
    }
}

impl Node {
    pub fn at_path(&self, path: &str) -> Result<Node, WzError> {
        if path.is_empty() {
            return Err(WzError::NodeNotFound(path.to_string()));
        }

        let segments: Vec<&str> = path.split('/').collect();

        let mut current = {
            let guard = self.wz_node.read().map_err(|_| WzError::LockPoisoned)?;
            guard
                .at(segments[0])
                .ok_or_else(|| WzError::NodeNotFound(path.to_string()))?
        };

        if segments[0].ends_with(".img") {
            wz_reader::util::node_util::parse_node(&current)?;
        }

        for &segment in &segments[1..] {
            current = {
                let guard = current.read().map_err(|_| WzError::LockPoisoned)?;
                guard
                    .at(segment)
                    .ok_or_else(|| WzError::NodeNotFound(path.to_string()))?
            };

            if segment.ends_with(".img") {
                wz_reader::util::node_util::parse_node(&current)?;
            }
        }

        Ok(current.into())
    }

    pub fn try_get(&self, name: &str) -> Option<Node> {
        let guard = self.wz_node.read().ok()?;
        let node: Node = guard.children.get(name)?.clone().into();
        Some(node)
    }

    pub fn children(&self) -> IndexMap<NodeName, Node> {
        let guard = self.wz_node.read().expect("lock poisoned");
        guard
            .children
            .iter()
            .map(|(k, v)| (k.clone().into(), v.clone().into()))
            .collect()
    }

    pub fn try_parse(&self) -> Result<&Self, WzError> {
        wz_reader::util::node_util::parse_node(&self.wz_node)?;
        Ok(self)
    }

    pub fn has(&self, name: &str) -> bool {
        self.wz_node
            .read()
            .expect("lock poisoned")
            .children
            .contains_key(name)
    }

    pub fn path(&self) -> String {
        self.wz_node
            .read()
            .expect("lock poisoned")
            .get_path_from_root()
            .to_string()
    }

    pub fn name(&self) -> String {
        self.wz_node
            .read()
            .expect("lock poisoned")
            .name
            .to_string()
    }

    pub fn has_image_data(&self) -> bool {
        let guard = match self.wz_node.read() {
            Ok(g) => g,
            Err(_) => return false,
        };
        if guard.try_as_png().is_some() {
            return true;
        }
        drop(guard);
        self.has("_inlink") || self.has("_outlink")
    }

    pub fn to_payload_depth(&self, depth: usize) -> Result<serde_json::Value, crate::error::WzError> {
        let guard = self.wz_node.read().map_err(|_| crate::error::WzError::LockPoisoned)?;
        let node_path = guard.get_path_from_root().to_string();
        let name = guard.name.to_string();
        drop(guard);

        let parent_path = if node_path.is_empty() || node_path == name {
            String::new()
        } else if let Some(stripped) = node_path.strip_suffix(&format!("/{}", name)) {
            stripped.to_string()
        } else {
            String::new()
        };

        self.to_payload_depth_inner(&parent_path, &name, &node_path, depth)
    }

    #[allow(clippy::too_many_arguments)]
    fn to_payload_depth_inner(
        &self,
        _parent_path: &str,
        name: &str,
        path: &str,
        depth: usize,
    ) -> Result<serde_json::Value, crate::error::WzError> {
        let guard = self.wz_node.read().map_err(|_| crate::error::WzError::LockPoisoned)?;
        let has_image = guard.try_as_png().is_some()
            || self.has("_inlink")
            || self.has("_outlink");

        let kind = if !guard.children.is_empty() {
            "container"
        } else if guard.try_as_png().is_some() {
            "png"
        } else if guard.try_as_short().is_some() {
            "short"
        } else if guard.try_as_int().is_some() {
            "int"
        } else if guard.try_as_long().is_some() {
            "long"
        } else if guard.try_as_string().is_some() {
            "string"
        } else if guard.try_as_float().is_some() || guard.try_as_double().is_some() {
            "float"
        } else if guard.try_as_vector2d().is_some() {
            "vector"
        } else if guard.try_as_image().is_some() {
            "image"
        } else {
            "property"
        };

        let value = if !guard.children.is_empty() {
            None
        } else if let Some(v) = guard.try_as_short() {
            Some(json!(v))
        } else if let Some(v) = guard.try_as_int() {
            Some(json!(v))
        } else if let Some(v) = guard.try_as_long() {
            Some(json!(v))
        } else if let Some(v) = guard.try_as_string() {
            v.get_string().ok().map(|s| json!(s))
        } else if let Some(v) = guard.try_as_float() {
            Some(json!(v))
        } else if let Some(v) = guard.try_as_double() {
            Some(json!(v))
        } else if let Some(v) = guard.try_as_vector2d() {
            Some(json!({"x": v.0, "y": v.1}))
        } else {
            None
        };

        drop(guard);

        if depth == 0 || kind != "container" {
            let mut val = json!({
                "name": name,
                "path": path,
                "kind": kind,
            });
            if let Some(v) = value {
                val["value"] = v;
            }
            if has_image {
                val["has_image"] = serde_json::Value::Bool(true);
            }
            return Ok(val);
        }

        let mut children: Vec<serde_json::Value> = self
            .children()
            .into_iter()
            .filter_map(|(child_name, child)| {
                let child_path = if path.is_empty() {
                    child_name.as_str().to_string()
                } else {
                    format!("{}/{}", path, child_name.as_str())
                };
                child.to_payload_depth_inner(path, child_name.as_str(), &child_path, depth - 1)
                    .ok()
            })
            .collect();

        children.sort_by(|a, b| {
            let na = a["name"].as_str().unwrap_or("");
            let nb = b["name"].as_str().unwrap_or("");
            let (na_num, nb_num) = (na.parse::<f64>(), nb.parse::<f64>());
            match (na_num, nb_num) {
                (Ok(a), Ok(b)) => a.total_cmp(&b),
                (Ok(_), Err(_)) => std::cmp::Ordering::Less,
                (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
                (Err(_), Err(_)) => na.cmp(nb),
            }
        });

        let mut val = json!({
            "name": name,
            "path": path,
            "kind": kind,
            "children": children,
        });
        if let Some(v) = value {
            val["value"] = v;
        }
        if has_image {
            val["has_image"] = serde_json::Value::Bool(true);
        }
        Ok(val)
    }

    pub fn read_pos(&self) -> Result<Vector2D, WzError> {
        let x: f32 = self.at_path("x")?.try_into()?;
        let y: f32 = self.at_path("y")?.try_into()?;
        Ok(Vector2D { x, y: -(y as f32) })
    }

    pub fn read_origin(&self, img_node: &Node) -> Result<Vector2D, WzError> {
        let guard = self.wz_node.read().map_err(|_| WzError::LockPoisoned)?;
        let &wz_reader::property::Vector2D(x, y) = guard
            .try_as_vector2d()
            .ok_or(WzError::TypeMismatch("Vector2D"))?;
        let mut y_f = y as f32;

        drop(guard);

        if let Ok(img_guard) = img_node.wz_node.read() {
            if let Some(png) = img_guard.try_as_png() {
                let h = png.height as f32;
                if h % 2.0 == 1.0 && y_f.abs() != 0.0 && y_f.abs() != h {
                    y_f -= 1.0;
                }
                y_f = h - y_f;
            }
        }

        Ok(Vector2D { x: x as f32, y: y_f })
    }

    pub fn read_pos_n(&self, n: u8) -> Result<Vector2D, WzError> {
        let x: f32 = self.at_path(&format!("x{n}"))?.try_into()?;
        let y: f32 = self.at_path(&format!("y{n}"))?.try_into()?;
        Ok(Vector2D { x: x as f32, y: -(y as f32) })
    }

    pub fn get_or<T: TryFrom<Node, Error = WzError>>(&self, path: &str, default: T) -> T {
        self.at_path(path)
            .ok()
            .and_then(|n| T::try_from(n).ok())
            .unwrap_or_else(|| {
                warn!(
                    "get_or: node '{}' missing/type-mismatch for '{}', using default",
                    self.path(),
                    path
                );
                default
            })
    }

    pub fn get_opt<T: TryFrom<Node, Error = WzError>>(&self, path: &str) -> Option<T> {
        self.at_path(path).ok().and_then(|n| T::try_from(n).ok())
    }

    pub fn required<T: TryFrom<Node, Error = WzError>>(&self, path: &str) -> T {
        self.at_path(path)
            .unwrap_or_else(|_| panic!("required child '{path}' not found"))
            .try_into()
            .unwrap_or_else(|_| panic!("required child '{path}' type mismatch"))
    }

    #[cfg(feature = "image-data")]
    pub fn extract_image(&self) -> Result<DynamicImage, WzError> {
        if let Some(inlink_node) = self.try_get("_inlink") {
            let path: String = inlink_node.try_into()?;
            let target = wz_reader::util::node_util::resolve_inlink(&path, &self.wz_node)
                .ok_or_else(|| WzError::NodeNotFound(format!("_inlink not found: {path}")))?;
            return Node::from(target).extract_image();
        }
        if let Some(outlink_node) = self.try_get("_outlink") {
            let path: String = outlink_node.try_into()?;
            let target = wz_reader::util::node_util::resolve_outlink(&path, &self.wz_node, true)
                .ok_or_else(|| WzError::NodeNotFound(format!("_outlink not found: {path}")))?;
            return Node::from(target).extract_image();
        }

        let guard = self.wz_node.read().map_err(|_| WzError::LockPoisoned)?;
        if let Some(png) = guard.try_as_png() {
            return png.extract_png()
                .map_err(|_| WzError::ValueError("failed to extract PNG".into()));
        }
        if guard.try_as_image().is_some() {
            return self.find_child_image();
        }
        Err(WzError::TypeMismatch("PNG image"))
    }

    fn find_child_image(&self) -> Result<DynamicImage, WzError> {
        for candidate in &["info/iconRaw", "info/icon", "canvas"] {
            if let Ok(child) = self.at_path(candidate) {
                if let Ok(img) = child.extract_image() {
                    return Ok(img);
                }
            }
        }
        Err(WzError::ValueError("no image found in container node".to_string()))
    }

    pub fn ordered_children(&self) -> Result<Vec<(String, Node)>, WzError> {
        self.try_parse()?;
        let guard = self.wz_node.read().map_err(|_| WzError::LockPoisoned)?;
        if let Some(image) = guard.try_as_image() {
            if let Ok((children, _)) = image.resolve_children(None) {
                return Ok(children
                    .into_iter()
                    .map(|(name, node)| (name.to_string(), Node { wz_node: node }))
                    .collect());
            }
        }
        Ok(self
            .children()
            .into_iter()
            .map(|(name, node)| (name.to_string(), node))
            .collect())
    }
}

impl TryFrom<Node> for i32 {
    type Error = WzError;
    fn try_from(node: Node) -> Result<Self, Self::Error> {
        let guard = node.wz_node.read().map_err(|_| WzError::LockPoisoned)?;
        guard
            .try_as_int()
            .copied()
            .or_else(|| guard.try_as_string()?.get_string().ok()?.parse().ok())
            .ok_or(WzError::TypeMismatch("i32"))
    }
}

impl TryFrom<Node> for f32 {
    type Error = WzError;
    fn try_from(node: Node) -> Result<Self, Self::Error> {
        {
            let guard = node.wz_node.read().map_err(|_| WzError::LockPoisoned)?;
            if let Some(v) = guard.try_as_float() {
                return Ok(*v);
            }
            if let Some(v) = guard.try_as_double() {
                return Ok(*v as f32);
            }
        }
        let value: i32 = node.try_into()?;
        Ok(value as f32)
    }
}

impl TryFrom<Node> for String {
    type Error = WzError;
    fn try_from(node: Node) -> Result<Self, Self::Error> {
        let guard = node.wz_node.read().map_err(|_| WzError::LockPoisoned)?;
        let wz_string = guard
            .try_as_string()
            .ok_or(WzError::TypeMismatch("String"))?;
        wz_string
            .get_string()
            .map_err(|_| WzError::ValueError("failed to decode string".into()))
    }
}

#[cfg(feature = "image-data")]
impl TryFrom<Node> for DynamicImage {
    type Error = WzError;
    fn try_from(node: Node) -> Result<Self, Self::Error> {
        let guard = node.wz_node.read().map_err(|_| WzError::LockPoisoned)?;
        let png = guard
            .try_as_png()
            .ok_or(WzError::TypeMismatch("PNG image"))?;
        png.extract_png()
            .map_err(|_| WzError::ValueError("failed to extract PNG".into()))
    }
}

impl TryFrom<Node> for bool {
    type Error = WzError;
    fn try_from(node: Node) -> Result<Self, Self::Error> {
        let value: i32 = node.try_into()?;
        Ok(value != 0)
    }
}

impl TryFrom<Node> for wz_reader::property::Vector2D {
    type Error = WzError;
    fn try_from(node: Node) -> Result<Self, Self::Error> {
        let guard = node.wz_node.read().map_err(|_| WzError::LockPoisoned)?;
        guard
            .try_as_vector2d()
            .copied()
            .ok_or(WzError::TypeMismatch("Vector2D"))
    }
}

impl<T: TryFrom<Node, Error = WzError>> TryFrom<Node> for Vec<T> {
    type Error = WzError;
    fn try_from(value: Node) -> Result<Self, Self::Error> {
        Ok(value
            .children()
            .into_iter()
            .filter(|(key, _)| key.to_string().parse::<u32>().is_ok())
            .filter_map(|(_, node)| node.try_into().ok())
            .collect())
    }
}

impl TryFrom<NodeName> for i32 {
    type Error = std::num::ParseIntError;
    fn try_from(key: NodeName) -> Result<Self, Self::Error> {
        key.wz_name.to_string().parse::<i32>()
    }
}

impl From<NodeName> for String {
    fn from(key: NodeName) -> Self {
        key.wz_name.to_string()
    }
}

impl<T: TryFrom<Node, Error = WzError>, K: TryFrom<NodeName>> TryFrom<Node> for Vec<(K, T)> {
    type Error = WzError;
    fn try_from(value: Node) -> Result<Self, Self::Error> {
        Ok(value
            .children()
            .into_iter()
            .filter_map(|(key, node)| Some((K::try_from(key).ok()?, node.try_into().ok()?)))
            .collect())
    }
}

impl<T: TryFrom<Node, Error = WzError>, K: TryFrom<NodeName> + Hash + Eq> TryFrom<Node> for HashMap<K, T> {
    type Error = WzError;
    fn try_from(value: Node) -> Result<Self, Self::Error> {
        Ok(value
            .children()
            .into_iter()
            .filter_map(|(key, node)| Some((K::try_from(key).ok()?, node.try_into().ok()?)))
            .collect())
    }
}

impl<T: TryFrom<Node, Error = WzError>, K: TryFrom<NodeName> + Hash + Eq> TryFrom<Node> for IndexMap<K, T> {
    type Error = WzError;
    fn try_from(value: Node) -> Result<Self, Self::Error> {
        Ok(value
            .children()
            .into_iter()
            .filter_map(|(key, node)| Some((K::try_from(key).ok()?, node.try_into().ok()?)))
            .collect())
    }
}

/// A serializable representation of a WZ node tree, used for HTTP transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePayload {
    pub name: String,
    pub value: Option<serde_json::Value>,
    pub children: Vec<NodePayload>,
}

impl Node {
    /// Convert this node to a serializable payload, limited to `depth` levels.
    pub fn to_node_payload(&self, depth: usize) -> Result<NodePayload, WzError> {
        self._to_node_payload(depth, 0)
    }

    fn _to_node_payload(&self, depth: usize, current: usize) -> Result<NodePayload, WzError> {
        let name = self.name();
        let value = self.extract_value().ok();

        let children = if current < depth {
            self.children()
                .into_iter()
                .filter_map(|(_, child)| child._to_node_payload(depth, current + 1).ok())
                .collect()
        } else {
            vec![]
        };

        Ok(NodePayload { name, value, children })
    }

    fn extract_value(&self) -> Result<serde_json::Value, WzError> {
        let guard = self.wz_node.read().map_err(|_| WzError::LockPoisoned)?;
        use wz_reader::WzObjectType;
        match &guard.object_type {
            WzObjectType::Value(v) => Ok(serde_json::to_value(v).unwrap_or(serde_json::Value::Null)),
            WzObjectType::Property(p) => Ok(serde_json::to_value(p).unwrap_or(serde_json::Value::Null)),
            _ => Ok(serde_json::Value::Null),
        }
    }
}

// ═════════════════════════════════════════════════════════════
//  WzNode trait implementation (shared interface with JsonNode)
// ═════════════════════════════════════════════════════════════

use crate::node_trait::{WzNode, TryFromNode};

impl WzNode for Node {
    fn at_path(&self, path: &str) -> Result<Self, WzError> { Node::at_path(self, path) }
    fn children(&self) -> IndexMap<String, Self> {
        Node::children(self).into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }
    fn try_get(&self, name: &str) -> Option<Self> { Node::try_get(self, name) }
    fn has(&self, name: &str) -> bool { Node::has(self, name) }
    fn name(&self) -> String { Node::name(self) }
    fn path(&self) -> String { Node::path(self) }
    fn read_pos(&self) -> Result<Vector2D, WzError> { Node::read_pos(self) }
    fn read_pos_n(&self, n: u8) -> Result<Vector2D, WzError> { Node::read_pos_n(self, n) }
    fn read_origin(&self, img_node: &Self) -> Result<Vector2D, WzError> { Node::read_origin(self, img_node) }
    fn ordered_children(&self) -> Result<Vec<(String, Self)>, WzError> { Node::ordered_children(self) }
    fn has_image_data(&self) -> bool { Node::has_image_data(self) }
    fn extract_image(&self) -> Result<DynamicImage, WzError> { Node::extract_image(self) }
}

impl TryFromNode<Node> for i32 {
    fn try_from_node(node: Node) -> Result<Self, WzError> { i32::try_from(node) }
}
impl TryFromNode<Node> for f32 {
    fn try_from_node(node: Node) -> Result<Self, WzError> { f32::try_from(node) }
}
impl TryFromNode<Node> for String {
    fn try_from_node(node: Node) -> Result<Self, WzError> { String::try_from(node) }
}
impl TryFromNode<Node> for bool {
    fn try_from_node(node: Node) -> Result<Self, WzError> { bool::try_from(node) }
}
impl TryFromNode<Node> for wz_reader::property::Vector2D {
    fn try_from_node(node: Node) -> Result<Self, WzError> { wz_reader::property::Vector2D::try_from(node) }
}


