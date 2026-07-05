use image::DynamicImage;
use indexmap::IndexMap;

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

    /// Convert this node to a typed value, like `.try_into()` but using TryFromNode.
    fn into_val<T: TryFromNode<Self>>(self) -> Result<T, WzError> {
        T::try_from_node(self)
    }

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
}

/// Marker trait for `WzNode` types that support scalar conversions.
/// Implemented automatically for any `N: WzNode` where the four basic
/// scalar conversions exist.  Data loaders bound on this instead of
/// repeating `where i32: TryFromNode<N>, ...` everywhere.
pub trait WzNodeConversions: WzNode
where
    i32: TryFromNode<Self>,
    f32: TryFromNode<Self>,
    String: TryFromNode<Self>,
    bool: TryFromNode<Self>,
{
}

impl<N: WzNode> WzNodeConversions for N
where
    i32: TryFromNode<N>,
    f32: TryFromNode<N>,
    String: TryFromNode<N>,
    bool: TryFromNode<N>,
{
}
