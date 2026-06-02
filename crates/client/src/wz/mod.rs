pub mod asset_loader;
pub mod asset_source;

use image::DynamicImage;
use bevy::prelude::*;
use indexmap::{Equivalent, IndexMap};
use std::collections::HashMap;
use std::hash::Hash;
use std::num::ParseIntError;
use std::sync::Arc;
use thiserror::Error;
use wz_reader::{property::Vector2D, WzNodeArc};
use wz_reader::{WzNodeCast, WzNodeName};

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
    let wz_node = wz_reader::util::resolve_base("./wz/Base.wz", None)?;
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
        guard.children
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
        self.wz_node.read().expect("lock poisoned").children.contains_key(name)
    }

    pub fn path(&self) -> String {
        self.wz_node.read().expect("lock poisoned").get_full_path().to_string()
    }

    /// Reads scalar `x` and `y` children and returns a Bevy-space `Vec2`
    /// (Y negated from WZ). Returns `NodeFound` if either child is missing.
    pub fn read_pos(&self) -> Result<Vec2, NodeError> {
        let x: f32 = self.at_path("x")?.try_into()?;
        let y: f32 = self.at_path("y")?.try_into()?;
        Ok(Vec2 { x, y: -y })
    }

    /// Reads scalar `x{n}` and `y{n}` children (e.g. `x1`/`y1`, `x2`/`y2`
    /// for footholds and areas) and returns a Bevy-space `Vec2`.
    pub fn read_pos_n(&self, n: u8) -> Result<Vec2, NodeError> {
        let x: f32 = self.at_path(&format!("x{n}"))?.try_into()?;
        let y: f32 = self.at_path(&format!("y{n}"))?.try_into()?;
        Ok(Vec2 { x, y: -y })
    }
}

/// Reads a WZ `Vector2D` and returns it in Bevy-space coordinates.
///
/// WZ stores 2D pixel coordinates with Y increasing downward. Bevy uses
/// Y-up. Every `Vector2D` in MapleStory WZ data is a pixel coordinate
/// (origins, connection points, etc.), so this impl applies the WZ\u2192Bevy
/// Y negation at the boundary. All downstream consumers receive Bevy-space
/// values.
impl TryFrom<Node> for Vec2 {
    type Error = NodeError;

    fn try_from(node: Node) -> Result<Self, Self::Error> {
        let guard = node.wz_node.read().map_err(|_| NodeError::LockPoisoned)?;
        let Vector2D(x, y) = guard.try_as_vector2d().ok_or(NodeError::TypeMismatch("Vec2"))?;
        Ok(Vec2 {
            x: *x as f32,
            y: -(*y as f32),
        })
    }
}

impl TryFrom<Node> for i32 {
    type Error = NodeError;

    fn try_from(node: Node) -> Result<Self, Self::Error> {
        let guard = node.wz_node.read().map_err(|_| NodeError::LockPoisoned)?;
        guard.try_as_int()
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
        }
        let value: i32 = node.try_into()?;
        Ok(value as f32)
    }
}

impl TryFrom<Node> for String {
    type Error = NodeError;

    fn try_from(node: Node) -> Result<Self, Self::Error> {
        let guard = node.wz_node.read().map_err(|_| NodeError::LockPoisoned)?;
        let wz_string = guard.try_as_string().ok_or(NodeError::TypeMismatch("String"))?;
        wz_string.get_string().map_err(|_| NodeError::ValueError("failed to decode string".into()))
    }
}

impl TryFrom<Node> for DynamicImage {
    type Error = NodeError;

    fn try_from(node: Node) -> Result<Self, Self::Error> {
        let guard = node.wz_node.read().map_err(|_| NodeError::LockPoisoned)?;
        let png = guard.try_as_png().ok_or(NodeError::TypeMismatch("PNG image"))?;
        png.extract_png().map_err(|_| NodeError::ValueError("failed to extract PNG".into()))
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
