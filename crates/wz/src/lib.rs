use image::DynamicImage;
use indexmap::{Equivalent, IndexMap};
use std::collections::HashMap;
use std::hash::Hash;
use std::num::ParseIntError;
use std::sync::Arc;
use thiserror::Error;
pub use wz_reader::property::Vector2D;
use wz_reader::{WzNodeArc, WzNodeCast, WzNodeName};

#[derive(Debug, Error)]
pub enum NodeError {
    #[error("node not found")]
    NodeNotFound,
    #[error("parse error: {0}")]
    WzError(#[from] wz_reader::node::Error),
    #[error("lock poisoned")]
    LockPoisoned,
    #[error("type mismatch: expected {0}")]
    TypeMismatch(&'static str),
    #[error("value error: {0}")]
    ValueError(String),
}

pub fn resolve_base() -> Result<Node, std::io::Error> {
    let path = std::env::var("WZ_PATH").unwrap_or_else(|_| "./wz/Base.wz".to_string());
    let wz_node = wz_reader::util::resolve_base(&path, None)?;
    Ok(wz_node.into())
}

static WZ_BASE: std::sync::OnceLock<Node> = std::sync::OnceLock::new();

/// Returns a cached WZ base node. First call resolves and caches it;
/// subsequent calls are instant (atomic load).
pub fn get_cached_base() -> &'static Node {
    WZ_BASE.get_or_init(|| resolve_base().expect("resolve_base failed"))
}

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

impl Equivalent<NodeName> for str {
    fn equivalent(&self, key: &NodeName) -> bool {
        self == key.as_str()
    }
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
    pub fn at_path(&self, path: &str) -> Result<Node, NodeError> {
        if path.is_empty() {
            return Err(NodeError::NodeNotFound);
        }

        let segments: Vec<&str> = path.split('/').collect();

        if segments.len() == 1 && !path.ends_with(".img") {
            return Ok(self.try_get(path).ok_or(NodeError::NodeNotFound)?);
        }

        let mut current = {
            let guard = self.wz_node.read().map_err(|_| NodeError::LockPoisoned)?;
            guard.at(segments[0]).ok_or(NodeError::NodeNotFound)?
        };

        if segments[0].ends_with(".img") {
            wz_reader::util::node_util::parse_node(&current)?;
        }

        for &segment in &segments[1..] {
            current = {
                let guard = current.read().map_err(|_| NodeError::LockPoisoned)?;
                guard.at(segment).ok_or(NodeError::NodeNotFound)?
            };

            if segment.ends_with(".img") {
                wz_reader::util::node_util::parse_node(&current)?;
            }
        }

        Ok(current.into())
    }

    pub fn get(&self, name: &str) -> Node {
        self.try_get(name).expect("child not found")
    }

    pub fn try_get(&self, name: &str) -> Option<Node> {
        let guard = self.wz_node.read().expect("lock poisoned");
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

    pub fn try_parse(&self) -> Result<&Self, NodeError> {
        wz_reader::util::node_util::parse_node(&self.wz_node)?;
        Ok(self)
    }

    pub fn parse(&self) -> &Self {
        self.try_parse().expect("parse failed")
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
            .get_full_path()
            .to_string()
    }

    /// Reads scalar `x` and `y` children and returns them with Y negated
    /// (converting from WZ's Y-down to the canonical Y-up convention).
    pub fn read_pos(&self) -> Result<Vector2D, NodeError> {
        let x: f32 = self.at_path("x")?.try_into()?;
        let y: f32 = self.at_path("y")?.try_into()?;
        Ok(Vector2D(x as i32, -(y as i32)))
    }

    /// Reads origin from this node (a native WZ `Vector2D` property) and
    /// converts to a bottom-left anchor in Y-up space, using the PNG height
    /// from `img_node`.
    ///
    /// WZ stores origin offsets as a `Vector2D` property with a top-left
    /// anchor in Y-down coordinates. This reads the image height from
    /// `img_node` and applies the anchor conversion: `result_y = h - y`
    /// (top-left Y-down → bottom-left Y-up), with a half-pixel correction
    /// for odd-height images.
    pub fn read_origin(&self, img_node: &Node) -> Result<Vector2D, NodeError> {
        let guard = self.wz_node.read().map_err(|_| NodeError::LockPoisoned)?;
        let &Vector2D(x, y) = guard
            .try_as_vector2d()
            .ok_or(NodeError::TypeMismatch("Vector2D"))?;
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

        Ok(Vector2D(x, y_f as i32))
    }

    /// Reads scalar `x{n}` and `y{n}` children (e.g. `x1`/`y1`, `x2`/`y2`
    /// for footholds and areas) with Y negated (converting from WZ's Y-down
    /// to the canonical Y-up convention).
    pub fn read_pos_n(&self, n: u8) -> Result<Vector2D, NodeError> {
        let x: f32 = self.at_path(&format!("x{n}"))?.try_into()?;
        let y: f32 = self.at_path(&format!("y{n}"))?.try_into()?;
        Ok(Vector2D(x as i32, -(y as i32)))
    }

    /// Reads a child node at `path` and tries to convert it to `T`,
    /// falling back to `default` on any failure.
    pub fn get_or<T: TryFrom<Node, Error = NodeError>>(&self, path: &str, default: T) -> T {
        self.at_path(path)
            .ok()
            .and_then(|n| T::try_from(n).ok())
            .unwrap_or(default)
    }

    /// Reads a child node at `path` and tries to convert it to `T`,
    /// returning `None` if the path doesn't exist or the type doesn't match.
    pub fn get_opt<T: TryFrom<Node, Error = NodeError>>(&self, path: &str) -> Option<T> {
        self.at_path(path).ok().and_then(|n| T::try_from(n).ok())
    }

    /// Reads a required child at `path` and converts to `T`.
    /// Panics with a descriptive message if the path doesn't exist
    /// or the type doesn't match.
    pub fn required<T: TryFrom<Node, Error = NodeError>>(&self, path: &str) -> T {
        self.at_path(path)
            .unwrap_or_else(|_| panic!("required child '{path}' not found"))
            .try_into()
            .unwrap_or_else(|_| panic!("required child '{path}' type mismatch"))
    }

    /// Resolve a relative path against this node's location (handles `..`).
    /// Used for `_inlink` resolution within the same `.img`.
    pub fn resolve_relative(&self, rel_path: &str) -> Result<Node, NodeError> {
        let current_path = self.path();
        let mut segs: Vec<&str> = current_path.split('/').collect();
        segs.pop();
        for part in rel_path.split('/') {
            match part {
                ".." => {
                    segs.pop();
                }
                "." => {}
                _ => segs.push(part),
            }
        }
        let absolute = segs.join("/");
        get_cached_base().at_path(&absolute)
    }

    /// Extract a PNG image from this node, transparently following
    /// `_inlink`, `_outlink`, and UOL references.
    pub fn extract_image(&self) -> Result<DynamicImage, NodeError> {
        if let Some(inlink_node) = self.try_get("_inlink") {
            let path: String = inlink_node.try_into()?;
            let resolved = self.resolve_relative(&path)?;
            return resolved.extract_image();
        }
        if let Some(outlink_node) = self.try_get("_outlink") {
            let path: String = outlink_node.try_into()?;
            let resolved = get_cached_base().at_path(&path)?;
            return resolved.extract_image();
        }
        let guard = self.wz_node.read().map_err(|_| NodeError::LockPoisoned)?;
        let png = guard
            .try_as_png()
            .ok_or(NodeError::TypeMismatch("PNG image"))?;
        png.extract_png()
            .map_err(|_| NodeError::ValueError("failed to extract PNG".into()))
    }

    /// Returns children in file order (as stored in the WZ archive), parsing
    /// the node first if needed. Falls back to unordered children if the node
    /// is not an Image type.
    pub fn ordered_children(&self) -> Result<Vec<(String, Node)>, NodeError> {
        self.try_parse()?;
        let guard = self.wz_node.read().map_err(|_| NodeError::LockPoisoned)?;
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
    type Error = NodeError;

    fn try_from(node: Node) -> Result<Self, Self::Error> {
        let guard = node.wz_node.read().map_err(|_| NodeError::LockPoisoned)?;
        guard
            .try_as_int()
            .copied()
            .or_else(|| guard.try_as_string()?.get_string().ok()?.parse().ok())
            .ok_or(NodeError::TypeMismatch("i32"))
    }
}

impl TryFrom<Node> for f32 {
    type Error = NodeError;

    fn try_from(node: Node) -> Result<Self, Self::Error> {
        {
            let guard = node.wz_node.read().map_err(|_| NodeError::LockPoisoned)?;
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
    type Error = NodeError;

    fn try_from(node: Node) -> Result<Self, Self::Error> {
        let guard = node.wz_node.read().map_err(|_| NodeError::LockPoisoned)?;
        let wz_string = guard
            .try_as_string()
            .ok_or(NodeError::TypeMismatch("String"))?;
        wz_string
            .get_string()
            .map_err(|_| NodeError::ValueError("failed to decode string".into()))
    }
}

impl TryFrom<Node> for DynamicImage {
    type Error = NodeError;

    fn try_from(node: Node) -> Result<Self, Self::Error> {
        let guard = node.wz_node.read().map_err(|_| NodeError::LockPoisoned)?;
        let png = guard
            .try_as_png()
            .ok_or(NodeError::TypeMismatch("PNG image"))?;
        png.extract_png()
            .map_err(|_| NodeError::ValueError("failed to extract PNG".into()))
    }
}

impl TryFrom<Node> for Arc<DynamicImage> {
    type Error = NodeError;

    fn try_from(node: Node) -> Result<Self, Self::Error> {
        let image: DynamicImage = node.try_into()?;
        Ok(Arc::new(image))
    }
}

impl TryFrom<Node> for bool {
    type Error = NodeError;

    fn try_from(node: Node) -> Result<Self, Self::Error> {
        let value: i32 = node.try_into()?;
        Ok(value != 0)
    }
}

impl TryFrom<Node> for Vector2D {
    type Error = NodeError;

    fn try_from(node: Node) -> Result<Self, Self::Error> {
        let guard = node.wz_node.read().map_err(|_| NodeError::LockPoisoned)?;
        guard
            .try_as_vector2d()
            .copied()
            .ok_or(NodeError::TypeMismatch("Vector2D"))
    }
}

impl<T: TryFrom<Node>> TryFrom<Node> for Vec<T> {
    type Error = NodeError;

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
    type Error = ParseIntError;
    fn try_from(key: NodeName) -> Result<Self, Self::Error> {
        key.wz_name.to_string().parse::<i32>()
    }
}

impl From<NodeName> for String {
    fn from(key: NodeName) -> Self {
        key.wz_name.to_string()
    }
}

impl<T: TryFrom<Node>, K: TryFrom<NodeName>> TryFrom<Node> for Vec<(K, T)> {
    type Error = NodeError;

    fn try_from(value: Node) -> Result<Self, Self::Error> {
        Ok(value
            .children()
            .into_iter()
            .filter_map(|(key, node)| Some((K::try_from(key).ok()?, node.try_into().ok()?)))
            .collect())
    }
}

impl<T: TryFrom<Node>, K: TryFrom<NodeName> + Hash + Eq> TryFrom<Node> for HashMap<K, T> {
    type Error = NodeError;

    fn try_from(value: Node) -> Result<Self, Self::Error> {
        Ok(value
            .children()
            .into_iter()
            .filter_map(|(key, node)| Some((K::try_from(key).ok()?, node.try_into().ok()?)))
            .collect())
    }
}

impl<T: TryFrom<Node>, K: TryFrom<NodeName> + Hash + Eq> TryFrom<Node> for IndexMap<K, T> {
    type Error = NodeError;

    fn try_from(value: Node) -> Result<Self, Self::Error> {
        Ok(value
            .children()
            .into_iter()
            .filter_map(|(key, node)| Some((K::try_from(key).ok()?, node.try_into().ok()?)))
            .collect())
    }
}


