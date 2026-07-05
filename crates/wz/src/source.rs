use crate::{Node, NodePayload, WzError, WzData};
use crate::error::NodeError;
use image::ImageFormat;
use std::collections::HashMap;
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
    fn json_node<'a>(&'a self, path: &'a str) -> WzSourceFuture<'a, crate::JsonNode>;
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

    fn json_node<'a>(&'a self, path: &'a str) -> WzSourceFuture<'a, crate::JsonNode> {
        Box::pin(async move {
            let payload = self.node_payload_depth(path, usize::MAX).await?;
            Ok(crate::JsonNode::from_payload(&payload))
        })
    }
}

pub fn default_source() -> &'static dyn WzSource {
    static SOURCE: std::sync::OnceLock<Box<dyn WzSource + Send + Sync>> = std::sync::OnceLock::new();
    SOURCE.get_or_init(|| make_source())
}

fn make_source() -> Box<dyn WzSource + Send + Sync> {
    #[cfg(not(target_arch = "wasm32"))]
    return Box::new(NativeWzSource);
    #[cfg(target_arch = "wasm32")]
    Box::new(HttpWzSource::new(WzData::global().api_base_url()))
}

impl WzSource for Box<dyn WzSource + Send + Sync + '_> {
    fn node<'a>(&'a self, path: &'a str) -> WzSourceFuture<'a, Node> {
        (**self).node(path)
    }
    fn node_payload<'a>(&'a self, path: &'a str) -> WzSourceFuture<'a, NodePayload> {
        (**self).node_payload(path)
    }
    fn node_payload_depth<'a>(&'a self, path: &'a str, depth: usize) -> WzSourceFuture<'a, NodePayload> {
        (**self).node_payload_depth(path, depth)
    }
    fn image_png<'a>(&'a self, path: &'a str) -> WzSourceFuture<'a, Vec<u8>> {
        (**self).image_png(path)
    }
    fn image_dynamic<'a>(&'a self, path: &'a str) -> WzSourceFuture<'a, image::DynamicImage> {
        (**self).image_dynamic(path)
    }
    fn json_node<'a>(&'a self, path: &'a str) -> WzSourceFuture<'a, crate::JsonNode> {
        (**self).json_node(path)
    }
}

#[cfg(target_arch = "wasm32")]
pub async fn wasm_fetch(url: &str) -> Result<Vec<u8>, js_sys::Error> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let opts = web_sys::RequestInit::new();
    opts.set_method("GET");
    let request = web_sys::Request::new_with_str_and_init(url, &opts)?;
    let window = web_sys::window().ok_or_else(|| js_sys::Error::new("no window"))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: web_sys::Response = resp_value.dyn_into().map_err(|_| js_sys::Error::new("not a response"))?;
    if !resp.ok() {
        return Err(js_sys::Error::new(&format!("HTTP {}", resp.status())));
    }
    let body = JsFuture::from(resp.array_buffer().map_err(|_| js_sys::Error::new("no array buffer"))?).await?;
    let uint8 = js_sys::Uint8Array::new(&body);
    let mut bytes = vec![0u8; uint8.length() as usize];
    uint8.copy_to(&mut bytes);
    Ok(bytes)
}

#[cfg(target_arch = "wasm32")]
pub async fn wasm_fetch_with_body(url: &str, body: &[u8]) -> Result<Vec<u8>, js_sys::Error> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    // Build headers as a JS object for RequestInit
    let headers = js_sys::Object::new();
    js_sys::Reflect::set(&headers, &wasm_bindgen::JsValue::from_str("Content-Type"), &wasm_bindgen::JsValue::from_str("application/json"))
        .map_err(|_| js_sys::Error::new("failed to set Content-Type"))?;

    let body_str = js_sys::JsString::from(String::from_utf8_lossy(body).as_ref());
    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&body_str);
    opts.set_headers_record_from_str_to_str(&headers);

    let window = web_sys::window().ok_or_else(|| js_sys::Error::new("no window"))?;
    let resp_value = JsFuture::from(window.fetch_with_str_and_init(url, &opts)).await?;
    let resp: web_sys::Response = resp_value.dyn_into().map_err(|_| js_sys::Error::new("not a response"))?;
    if !resp.ok() {
        return Err(js_sys::Error::new(&format!("HTTP {}", resp.status())));
    }
    let resp_body = JsFuture::from(resp.array_buffer().map_err(|_| js_sys::Error::new("no array buffer"))?).await?;
    let uint8 = js_sys::Uint8Array::new(&resp_body);
    let mut bytes = vec![0u8; uint8.length() as usize];
    uint8.copy_to(&mut bytes);
    Ok(bytes)
}

// ═══════════════════════════════════════════════════════════
//  Free-standing bundle loaders
// ═══════════════════════════════════════════════════════════

pub async fn load_physics() -> Result<std::sync::Arc<crate::PhysicsConstants>, WzSourceError> {
    #[cfg(not(target_arch = "wasm32"))]
    return Ok(WzData::global().load_physics()?);
    #[cfg(target_arch = "wasm32")]
    {
        let base = WzData::global().api_base_url().to_string();
        let url = format!("{}/wz/bdata/physics", base);
        let bytes = wasm_fetch(&url).await
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(
                e.as_string().unwrap_or_else(|| "unknown error".into()),
            )))?;
        let (data, _) = bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(e.to_string())))?;
        Ok(data)
    }
}

pub async fn load_zmap() -> Result<Vec<(String, usize)>, WzSourceError> {
    #[cfg(not(target_arch = "wasm32"))]
    return Ok(WzData::global().load_zmap()?);
    #[cfg(target_arch = "wasm32")]
    {
        let base = WzData::global().api_base_url().to_string();
        let url = format!("{}/wz/bdata/zmap", base);
        let bytes = wasm_fetch(&url).await
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(
                e.as_string().unwrap_or_else(|| "unknown error".into()),
            )))?;
        let (data, _) = bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(e.to_string())))?;
        Ok(data)
    }
}

pub async fn load_skill_database() -> Result<std::sync::Arc<crate::SkillDatabase>, WzSourceError> {
    #[cfg(not(target_arch = "wasm32"))]
    return Ok(WzData::global().load_skill_database()?);
    #[cfg(target_arch = "wasm32")]
    {
        let base = WzData::global().api_base_url().to_string();
        let url = format!("{}/wz/bdata/skill-database", base);
        let bytes = wasm_fetch(&url).await
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(
                e.as_string().unwrap_or_else(|| "unknown error".into()),
            )))?;
        let (data, _) = bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(e.to_string())))?;
        Ok(data)
    }
}

pub async fn load_job_catalog() -> Result<Vec<(u32, String)>, WzSourceError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let wz_data = WzData::global();
        let class_names = wz_data.list_children("Skill").map_err(|e| WzSourceError::Wz(e))?;
        let mut entries: Vec<(u32, String)> = Vec::new();
        for class_name in class_names {
            let Some(job_key) = class_name.strip_suffix(".img") else { continue };
            let Ok(job_id) = job_key.parse::<u32>() else { continue };
            let label: Option<String> = wz_data.read_string(&format!("String/Skill.img/{job_key}/bookName"));
            if let Some(label) = label {
                let label = label.trim().to_string();
                if !label.is_empty() {
                    entries.push((job_id, label));
                }
            }
        }
        entries.sort_by_key(|(id, _)| *id);
        Ok(entries)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let base = WzData::global().api_base_url().to_string();
        let url = format!("{}/wz/bdata/job-catalog", base);
        let bytes = wasm_fetch(&url).await
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(
                e.as_string().unwrap_or_else(|| "unknown error".into()),
            )))?;
        let (data, _) = bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(e.to_string())))?;
        Ok(data)
    }
}

pub async fn load_action_lists() -> Result<(Vec<String>, Vec<String>), WzSourceError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let wz_data = WzData::global();
        let basic: Vec<String> = wz_data
            .list_children("Character/00002001.img")
            .map_err(|e| WzSourceError::Wz(e))?
            .into_iter()
            .filter(|a| a != "info")
            .collect();
        let basic_set: std::collections::HashSet<&str> =
            basic.iter().map(|s| s.as_str()).collect();
        let all = wz_data
            .list_children("Character/00002000.img")
            .map_err(|e| WzSourceError::Wz(e))?
            .into_iter()
            .filter(|a| a != "info");
        let composite: Vec<String> = all.filter(|a| !basic_set.contains(a.as_str())).collect();
        Ok((basic, composite))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let base = WzData::global().api_base_url().to_string();
        let url = format!("{}/wz/bdata/action-lists", base);
        let bytes = wasm_fetch(&url).await
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(
                e.as_string().unwrap_or_else(|| "unknown error".into()),
            )))?;
        #[derive(serde::Deserialize)]
        struct ActionListsResponse {
            basic: Vec<String>,
            composite: Vec<String>,
        }
        let resp: ActionListsResponse = bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(e.to_string())))?
            .0;
        Ok((resp.basic, resp.composite))
    }
}

pub async fn load_portal_frames() -> Result<Vec<crate::PortalFrameData>, WzSourceError> {
    #[cfg(not(target_arch = "wasm32"))]
    return Ok(WzData::global().load_portal_frames()?);
    #[cfg(target_arch = "wasm32")]
    {
        let base = WzData::global().api_base_url().to_string();
        let url = format!("{}/wz/bdata/portal-frames", base);
        let bytes = wasm_fetch(&url).await
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(
                e.as_string().unwrap_or_else(|| "unknown error".into()),
            )))?;
        let (data, _) = bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(e.to_string())))?;
        Ok(data)
    }
}

pub async fn load_character_body(skin: u32, action: &str) -> Result<std::sync::Arc<crate::CharacterBody>, WzSourceError> {
    #[cfg(not(target_arch = "wasm32"))]
    return Ok(WzData::global().load_character_body(skin, action)?);
    #[cfg(target_arch = "wasm32")]
    {
        let base = WzData::global().api_base_url().to_string();
        let url = format!("{}/wz/bdata/character-body/{skin}/{action}", base);
        let bytes = wasm_fetch(&url).await
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(
                e.as_string().unwrap_or_else(|| "unknown error".into()),
            )))?;
        let (data, _) = bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(e.to_string())))?;
        Ok(data)
    }
}

pub async fn load_hair_body(hair_id: u32, action: &str) -> Result<std::sync::Arc<crate::HairBody>, WzSourceError> {
    #[cfg(not(target_arch = "wasm32"))]
    return Ok(WzData::global().load_hair_body(hair_id, action)?);
    #[cfg(target_arch = "wasm32")]
    {
        let base = WzData::global().api_base_url().to_string();
        let url = format!("{}/wz/bdata/hair-body/{hair_id}/{action}", base);
        let bytes = wasm_fetch(&url).await
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(
                e.as_string().unwrap_or_else(|| "unknown error".into()),
            )))?;
        let (data, _) = bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(e.to_string())))?;
        Ok(data)
    }
}

pub async fn load_equip_action(item_id: i32, action: &str) -> Result<std::sync::Arc<crate::EquipAction>, WzSourceError> {
    #[cfg(not(target_arch = "wasm32"))]
    return Ok(WzData::global().load_equip_action(item_id, action)?);
    #[cfg(target_arch = "wasm32")]
    {
        let base = WzData::global().api_base_url().to_string();
        let url = format!("{}/wz/bdata/equip-action/{item_id}/{action}", base);
        let bytes = wasm_fetch(&url).await
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(
                e.as_string().unwrap_or_else(|| "unknown error".into()),
            )))?;
        let (data, _) = bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(e.to_string())))?;
        Ok(data)
    }
}

pub async fn load_face_expression(face_id: u32, expression: &str) -> Result<std::sync::Arc<crate::FaceExpression>, WzSourceError> {
    #[cfg(not(target_arch = "wasm32"))]
    return Ok(WzData::global().load_face_expression(face_id, expression)?);
    #[cfg(target_arch = "wasm32")]
    {
        let base = WzData::global().api_base_url().to_string();
        let url = format!("{}/wz/bdata/face-expression/{face_id}/{expression}", base);
        let bytes = wasm_fetch(&url).await
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(
                e.as_string().unwrap_or_else(|| "unknown error".into()),
            )))?;
        let (data, _) = bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(e.to_string())))?;
        Ok(data)
    }
}

pub async fn load_origin(path: &str) -> Result<(f32, f32), WzSourceError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let v = WzData::global().load_origin(path)?;
        Ok((v.0, v.1))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let base = WzData::global().api_base_url().to_string();
        let url = format!("{}/wz/bdata/origin/{}", base, path);
        let bytes = wasm_fetch(&url).await
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(
                e.as_string().unwrap_or_else(|| "unknown error".into()),
            )))?;
        let (data, _) = bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(e.to_string())))?;
        Ok(data)
    }
}

pub async fn load_map_bundle(id: i32) -> Result<crate::MapBundle, WzSourceError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let data = (*WzData::global().load_map(id)?).clone();
        let paths = collect_map_image_paths(&data);
        let source = NativeWzSource;
        let images = load_images_png_impl(&source, &paths).await?;
        Ok(crate::MapBundle { data, images })
    }
    #[cfg(target_arch = "wasm32")]
    {
        let base = WzData::global().api_base_url().to_string();
        let url = format!("{}/wz/bundle/map/{}", base, id);
        let bytes = wasm_fetch(&url).await
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(
                e.as_string().unwrap_or_else(|| "unknown error".into()),
            )))?;
        let (data, _) = bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(e.to_string())))?;
        Ok(data)
    }
}

pub async fn load_mob_bundle(id: i32) -> Result<crate::MobBundle, WzSourceError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let data = (*WzData::global().load_mob(id)?).clone();
        let paths = collect_mob_image_paths(&data);
        let source = NativeWzSource;
        let images = load_images_png_impl(&source, &paths).await?;
        Ok(crate::MobBundle { data, images })
    }
    #[cfg(target_arch = "wasm32")]
    {
        let base = WzData::global().api_base_url().to_string();
        let url = format!("{}/wz/bundle/mob/{}", base, id);
        let bytes = wasm_fetch(&url).await
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(
                e.as_string().unwrap_or_else(|| "unknown error".into()),
            )))?;
        let (data, _) = bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(e.to_string())))?;
        Ok(data)
    }
}

pub async fn load_npc_bundle(id: i32) -> Result<crate::NpcBundle, WzSourceError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let data = (*WzData::global().load_npc(id)?).clone();
        let paths = collect_npc_image_paths(&data);
        let source = NativeWzSource;
        let images = load_images_png_impl(&source, &paths).await?;
        Ok(crate::NpcBundle { data, images })
    }
    #[cfg(target_arch = "wasm32")]
    {
        let base = WzData::global().api_base_url().to_string();
        let url = format!("{}/wz/bundle/npc/{}", base, id);
        let bytes = wasm_fetch(&url).await
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(
                e.as_string().unwrap_or_else(|| "unknown error".into()),
            )))?;
        let (data, _) = bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
            .map_err(|e| WzSourceError::Node(NodeError::ValueError(e.to_string())))?;
        Ok(data)
    }
}

pub async fn load_image_bundle(paths: &[String]) -> Result<crate::ImageBundle, WzSourceError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let source = NativeWzSource;
        let images = load_images_png_impl(&source, paths).await?;
        Ok(crate::ImageBundle { images, origins: std::collections::HashMap::new() })
    }
    #[cfg(target_arch = "wasm32")]
    {
        let source = default_source();
        let mut images = std::collections::HashMap::new();
        for path in paths {
            if let Ok(img) = source.image_dynamic(path).await {
                let mut png = Vec::new();
                let _ = img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png);
                images.insert(path.clone(), png);
            }
        }
        Ok(crate::ImageBundle { images, origins: std::collections::HashMap::new() })
    }
}

fn deduplicate_paths(paths: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    paths.retain(|p| seen.insert(p.clone()));
}

pub(crate) fn collect_map_image_paths(map: &crate::MapData) -> Vec<String> {
    let mut paths = Vec::new();
    for bg in &map.backgrounds {
        paths.push(bg.image_path.clone());
        for anim in &bg.animation_frames {
            paths.push(anim.image_path.clone());
        }
    }
    for layer in &map.layers {
        for tile in &layer.tiles {
            paths.push(tile.image_path.clone());
            for anim in &tile.animation_frames {
                paths.push(anim.image_path.clone());
            }
        }
        for obj in &layer.objs {
            paths.push(obj.image_path.clone());
            for anim in &obj.animation_frames {
                paths.push(anim.image_path.clone());
            }
        }
    }
    if let Some(ref mm) = map.minimap {
        paths.push(mm.image_path.clone());
    }
    deduplicate_paths(&mut paths);
    paths
}

pub(crate) fn collect_mob_image_paths(mob: &crate::MobData) -> Vec<String> {
    let mut paths = Vec::new();
    for action in mob.actions.values() {
        for frame in &action.frames {
            for part in &frame.parts {
                paths.push(part.image_path.clone());
            }
        }
    }
    deduplicate_paths(&mut paths);
    paths
}

pub(crate) fn collect_npc_image_paths(npc: &crate::NpcData) -> Vec<String> {
    let mut paths = Vec::new();
    for action in npc.actions.values() {
        for frame in &action.frames {
            paths.push(frame.image_path.clone());
        }
    }
    deduplicate_paths(&mut paths);
    paths
}

pub(crate) async fn load_images_png_impl<S: WzSource + Sync>(
    source: &S,
    paths: &[String],
) -> Result<std::collections::HashMap<String, Vec<u8>>, WzSourceError> {
    let mut images = std::collections::HashMap::new();
    for path in paths {
        if let Ok(png) = source.image_png(path).await {
            images.insert(path.clone(), png);
        }
    }
    Ok(images)
}
