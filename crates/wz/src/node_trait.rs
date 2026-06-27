use image::DynamicImage;
use indexmap::IndexMap;
use std::collections::HashMap;
use std::hash::Hash;

use crate::error::WzError;
use crate::vector2d::Vector2D;

/// Trait for WZ node types that data loaders can use.
/// Implemented by `Node` (native, backed by `wz_reader`) and `JsonNode` (WASM, backed by HTTP JSON).
pub trait WzNode: Clone + Sized {
    fn at_path(&self, path: &str) -> Result<Self, WzError>;
    fn children(&self) -> IndexMap<String, Self>;
    fn try_get(&self, name: &str) -> Option<Self>;
    fn has(&self, name: &str) -> bool;
    fn name(&self) -> String;
    fn path(&self) -> String;
    fn read_pos(&self) -> Result<Vector2D, WzError>;
    fn read_pos_n(&self, n: u8) -> Result<Vector2D, WzError>;
    fn read_origin(&self, img_node: &Self) -> Result<Vector2D, WzError>;
    fn ordered_children(&self) -> Result<Vec<(String, Self)>, WzError>;
    fn has_image_data(&self) -> bool;
    fn extract_image(&self) -> Result<DynamicImage, WzError>;

    fn get_opt<T: TryFromNode<Self>>(&self, path: &str) -> Option<T> {
        self.at_path(path).ok().and_then(|n| T::try_from_node(n).ok())
    }

    fn get_or<T: TryFromNode<Self>>(&self, path: &str, default: T) -> T {
        self.at_path(path).ok().and_then(|n| T::try_from_node(n).ok()).unwrap_or(default)
    }

    fn required<T: TryFromNode<Self>>(&self, path: &str) -> T {
        let node = self.at_path(path)
            .unwrap_or_else(|_| panic!("required child '{path}' not found"));
        T::try_from_node(node)
            .unwrap_or_else(|_| panic!("required child '{path}' type mismatch"))
    }
}

/// Trait for converting a WZ node to a scalar value.
pub trait TryFromNode<N: WzNode>: Sized {
    fn try_from_node(node: N) -> Result<Self, WzError>;
    fn try_into_node(self) -> Result<Self, WzError> where Self: Sized {
        Ok(self)
    }
}

// Blanket impl for Vec<T>
impl<N: WzNode, T: TryFromNode<N>> TryFromNode<N> for Vec<T> {
    fn try_from_node(value: N) -> Result<Self, WzError> {
        Ok(value
            .children()
            .into_iter()
            .filter(|(key, _)| key.parse::<u32>().is_ok())
            .filter_map(|(_, node)| T::try_from_node(node).ok())
            .collect())
    }
}

// Blanket impl for HashMap
impl<N: WzNode, T: TryFromNode<N>, K: TryFrom<String> + Hash + Eq> TryFromNode<N> for HashMap<K, T> {
    fn try_from_node(value: N) -> Result<Self, WzError> {
        Ok(value
            .children()
            .into_iter()
            .filter_map(|(key, node)| Some((K::try_from(key).ok()?, T::try_from_node(node).ok()?)))
            .collect())
    }
}

// Blanket impl for IndexMap
impl<N: WzNode, T: TryFromNode<N>, K: TryFrom<String> + Hash + Eq> TryFromNode<N> for IndexMap<K, T> {
    fn try_from_node(value: N) -> Result<Self, WzError> {
        Ok(value
            .children()
            .into_iter()
            .filter_map(|(key, node)| Some((K::try_from(key).ok()?, T::try_from_node(node).ok()?)))
            .collect())
    }
}
