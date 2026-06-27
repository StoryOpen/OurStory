use crate::error::NodeError;
use crate::NodePayload;
use crate::source::{WzSource, WzSourceError, WzSourceFuture};
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
            let bytes = fetch_bytes(&url).await
                .map_err(|e| WzSourceError::Node(NodeError::ValueError(e.to_string().as_string().unwrap_or_default())))?;
            let payload: NodePayload = serde_json::from_slice(&bytes)
                .map_err(|e| WzSourceError::Node(NodeError::ValueError(e.to_string())))?;
            Ok(payload)
        })
    }

    fn image_png<'a>(&'a self, path: &'a str) -> WzSourceFuture<'a, Vec<u8>> {
        Box::pin(async move {
            let url = format!("{}/wz/image/{}", self.base_url, path);
            fetch_bytes(&url).await
                .map_err(|e| WzSourceError::Node(NodeError::ValueError(e.to_string().as_string().unwrap_or_default())))
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
}

async fn fetch_bytes(url: &str) -> Result<Vec<u8>, js_sys::Error> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let opts = web_sys::RequestInit::new();
    opts.set_method("GET");
    let request = web_sys::Request::new_with_str_and_init(url, &opts)?;
    let window = web_sys::window().ok_or_else(|| js_sys::Error::new("no window"))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: web_sys::Response = resp_value.dyn_into().map_err(|_| js_sys::Error::new("not a response"))?;
    let ok = resp.ok();
    let status = resp.status();
    if !ok {
        return Err(js_sys::Error::new(&format!("HTTP {status}")));
    }
    let body = JsFuture::from(resp.array_buffer().map_err(|_| js_sys::Error::new("no array buffer"))?).await?;
    let uint8 = js_sys::Uint8Array::new(&body);
    let mut bytes = vec![0u8; uint8.length() as usize];
    uint8.copy_to(&mut bytes);
    Ok(bytes)
}
