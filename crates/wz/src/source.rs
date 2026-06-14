use std::io::Cursor;
use image::ImageFormat;

use crate::error::WzError;
use crate::get_cached_base;

pub struct NativeWzSource;

impl NativeWzSource {
    pub async fn node_payload_depth(path: &str, depth: usize) -> Result<serde_json::Value, WzError> {
        let node = get_cached_base().at_path(path)?;
        node.to_payload_depth(depth)
    }

    pub async fn image_png(path: &str) -> Result<Vec<u8>, WzError> {
        let node = get_cached_base().at_path(path)?;
        let img = node.extract_image()?;
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .map_err(|e| WzError::ValueError(format!("failed to encode PNG: {e}")))?;
        Ok(buf)
    }
}

pub trait WzSource {
    async fn node_payload_depth(&self, path: &str, depth: usize) -> Result<serde_json::Value, WzError>;
    async fn image_png(&self, path: &str) -> Result<Vec<u8>, WzError>;
}

impl WzSource for NativeWzSource {
    async fn node_payload_depth(&self, path: &str, depth: usize) -> Result<serde_json::Value, WzError> {
        Self::node_payload_depth(path, depth).await
    }

    async fn image_png(&self, path: &str) -> Result<Vec<u8>, WzError> {
        Self::image_png(path).await
    }
}
