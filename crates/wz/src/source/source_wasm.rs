use crate::error::NodeError;
use crate::NodePayload;
use crate::source::{WzSource, WzSourceError, WzSourceFuture, wasm_fetch};
use image::DynamicImage;

pub struct HttpWzSource {
    base_url: String,
}

impl HttpWzSource {
    pub fn new(base_url: impl Into<String>) -> Self {
        HttpWzSource { base_url: base_url.into() }
    }

    pub fn same_origin() -> Self {
        let origin = web_sys::window()
            .and_then(|w| w.location().origin().ok())
            .unwrap_or_default();
        HttpWzSource { base_url: origin }
    }
}

impl WzSource for HttpWzSource {
    fn node<'a>(&'a self, _path: &'a str) -> WzSourceFuture<'a, crate::Node> {
        Box::pin(async move {
            Err(WzSourceError::Node(NodeError::ValueError("HttpWzSource::node() not supported, use node_payload() instead".into())))
        })
    }

    fn node_payload<'a>(&'a self, path: &'a str) -> WzSourceFuture<'a, NodePayload> {
        self.node_payload_depth(path, usize::MAX)
    }

    fn node_payload_depth<'a>(&'a self, path: &'a str, depth: usize) -> WzSourceFuture<'a, NodePayload> {
        Box::pin(async move {
            let url = format!("{}/wz/node/{}?depth={}", self.base_url, path, depth);
            let bytes = wasm_fetch(&url).await
                .map_err(|e| WzSourceError::Node(NodeError::ValueError(e.as_string().unwrap_or_else(|| "unknown error".into()))))?;
            let payload: NodePayload = serde_json::from_slice(&bytes)
                .map_err(|e| WzSourceError::Node(NodeError::ValueError(e.to_string())))?;
            Ok(payload)
        })
    }

    fn image_png<'a>(&'a self, path: &'a str) -> WzSourceFuture<'a, Vec<u8>> {
        Box::pin(async move {
            let url = format!("{}/wz/image/{}", self.base_url, path);
            wasm_fetch(&url).await
                .map_err(|e| WzSourceError::Node(NodeError::ValueError(e.as_string().unwrap_or_else(|| "unknown error".into()))))
        })
    }

    fn image_dynamic<'a>(&'a self, path: &'a str) -> WzSourceFuture<'a, DynamicImage> {
        Box::pin(async move {
            let png = self.image_png(path).await?;
            let img = image::load_from_memory(&png)
                .map_err(|e| WzSourceError::Image(e))?;
            Ok(img)
        })
    }

    fn json_node<'a>(&'a self, path: &'a str) -> WzSourceFuture<'a, crate::JsonNode> {
        Box::pin(async move {
            let payload = self.node_payload_depth(path, usize::MAX).await?;
            Ok(crate::JsonNode::from_payload(&payload))
        })
    }
}
