use crate::{Node, NodePayload, WzError};
use crate::error::NodeError;
use image::ImageFormat;
use std::future::Future;
use std::pin::Pin;
use thiserror::Error;

#[cfg(target_arch = "wasm32")]
mod source_wasm;
#[cfg(target_arch = "wasm32")]
pub use source_wasm::HttpWzSource;

#[derive(Debug, Error)]
pub enum WzSourceError {
    #[error(transparent)]
    Node(#[from] NodeError),
    #[error("wz error: {0}")]
    Wz(#[from] WzError),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
}

#[cfg(not(target_arch = "wasm32"))]
pub type WzSourceFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, WzSourceError>> + Send + 'a>>;
#[cfg(target_arch = "wasm32")]
pub type WzSourceFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, WzSourceError>> + 'a>>;

pub trait WzSource {
    fn node<'a>(&'a self, path: &'a str) -> WzSourceFuture<'a, Node>;
    fn node_payload<'a>(&'a self, path: &'a str) -> WzSourceFuture<'a, NodePayload>;
    fn node_payload_depth<'a>(&'a self, path: &'a str, depth: usize) -> WzSourceFuture<'a, NodePayload>;
    fn image_png<'a>(&'a self, path: &'a str) -> WzSourceFuture<'a, Vec<u8>>;
    fn image_dynamic<'a>(&'a self, path: &'a str) -> WzSourceFuture<'a, image::DynamicImage>;
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, Default)]
pub struct NativeWzSource;

#[cfg(not(target_arch = "wasm32"))]
impl WzSource for NativeWzSource {
    fn node<'a>(&'a self, path: &'a str) -> WzSourceFuture<'a, Node> {
        Box::pin(async move { Ok(crate::get_cached_base().at_path(path)?) })
    }

    fn node_payload<'a>(&'a self, path: &'a str) -> WzSourceFuture<'a, NodePayload> {
        self.node_payload_depth(path, usize::MAX)
    }

    fn node_payload_depth<'a>(&'a self, path: &'a str, depth: usize) -> WzSourceFuture<'a, NodePayload> {
        Box::pin(async move { Ok(crate::get_cached_base().at_path(path)?.to_node_payload(depth)?) })
    }

    fn image_png<'a>(&'a self, path: &'a str) -> WzSourceFuture<'a, Vec<u8>> {
        Box::pin(async move {
            let node = crate::get_cached_base().at_path(path)?;
            let image: image::DynamicImage = node.try_into()?;
            let mut cursor = std::io::Cursor::new(Vec::new());
            image.write_to(&mut cursor, ImageFormat::Png)?;
            Ok(cursor.into_inner())
        })
    }

    fn image_dynamic<'a>(&'a self, path: &'a str) -> WzSourceFuture<'a, image::DynamicImage> {
        Box::pin(async move {
            let node = crate::get_cached_base().at_path(path)?;
            Ok(node.try_into()?)
        })
    }
}

pub fn default_source() -> impl WzSource {
    #[cfg(not(target_arch = "wasm32"))]
    return NativeWzSource;
    #[cfg(target_arch = "wasm32")]
    return HttpWzSource::new("http://127.0.0.1:3000");
}
