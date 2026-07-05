use log::warn;
use std::collections::HashMap;
use crate::error::WzError;
use crate::node_trait::{WzNode, TryFromNode};
use crate::vector2d::Vector2D;

// ── BodyPart ──

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BodyPart {
    pub image_path: String,
    pub origin: Vector2D,
    pub map: HashMap<String, Vector2D>,
    pub z: f32,
    pub part_name: String,
    pub slot: Option<String>,
}

// ── BodyFrame ──

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BodyFrame {
    pub parts: Vec<BodyPart>,
    pub delay: u32,
}

// ── CharacterBody ──

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CharacterBody {
    pub frames: Vec<BodyFrame>,
}

impl CharacterBody {
    pub fn load<N: WzNode>(base: &N, skin_suffix: u32, action: &str) -> Result<Self, WzError>
    where i32: TryFromNode<N>, f32: TryFromNode<N>, String: TryFromNode<N>, bool: TryFromNode<N>, wz_reader::property::Vector2D: TryFromNode<N>
    {
        let body_path = format!("Character/0000{:04}.img/{}", skin_suffix, action);
        let head_path = format!("Character/0001{:04}.img/{}", skin_suffix, action);

        let body_node = base.at_path(&body_path)?;
        let body_frames = Self::load_frames(base, &body_path, &body_node)?;

        let frames = match base.at_path(&head_path).ok() {
            Some(head_node) => {
                let head_frames = Self::load_frames(base, &head_path, &head_node)?;
                if body_frames.len() != head_frames.len() {
                    panic!(
                        "CharacterBody: frame count mismatch for action '{}': body={} head={}",
                        action,
                        body_frames.len(),
                        head_frames.len(),
                    );
                }
                let mut merged = Vec::with_capacity(body_frames.len());
                for (i, mut frame) in body_frames.into_iter().enumerate() {
                    frame.parts.extend(head_frames[i].parts.clone());
                    merged.push(frame);
                }
                merged
            }
            None => body_frames,
        };

        Ok(CharacterBody { frames })
    }

    fn load_frames<N: WzNode>(base: &N, action_path: &str, action_node: &N) -> Result<Vec<BodyFrame>, WzError>
    where i32: TryFromNode<N>, f32: TryFromNode<N>, String: TryFromNode<N>, bool: TryFromNode<N>, wz_reader::property::Vector2D: TryFromNode<N>
    {
        let frame_count = action_node.children().len();
        if frame_count == 0 {
            return Ok(Vec::new());
        }
        let mut frames = Vec::with_capacity(frame_count);
        for frame_idx in 0..frame_count as u32 {
            if let Some(frame) = Self::load_single_frame(base, action_path, frame_idx) {
                frames.push(frame);
            }
        }
        Ok(frames)
    }

    fn load_single_frame<N: WzNode>(base: &N, action_path: &str, frame_idx: u32) -> Option<BodyFrame>
    where i32: TryFromNode<N>, f32: TryFromNode<N>, String: TryFromNode<N>, bool: TryFromNode<N>, wz_reader::property::Vector2D: TryFromNode<N>
    {
        let frame_path = format!("{}/{}", action_path, frame_idx);
        let frame_node = base.at_path(&frame_path).ok()?;

        let delay: i32 = frame_node.get_or("delay", 100);

        // Frame-reference: delegate to another action's frame
        if let (Ok(action_node), Ok(frame_node_val)) = (
            frame_node.at_path("action"),
            frame_node.at_path("frame"),
        ) {
            let ref_action: String = action_node.into_val().ok()?;
            let ref_frame: i32 = frame_node_val.into_val().ok()?;
            let ref_action_path = action_path
                .rsplit_once('/')
                .map(|(parent, _)| format!("{}/{}", parent, ref_action))
                .unwrap_or_else(|| format!("{}/{}", action_path, ref_action));
            let mut frame = Self::load_single_frame(base, &ref_action_path, ref_frame as u32)?;
            frame.delay = delay.unsigned_abs();
            return Some(frame);
        }

        let mut parts = Vec::new();
        for (child_name, _) in frame_node.children() {
            let cn = child_name.as_str();
            if cn == "delay" || cn == "face" {
                continue;
            }
            if let Some(part) = load_body_part(&frame_node, cn) {
                parts.push(part);
            }
        }

        Some(BodyFrame { parts, delay: delay.unsigned_abs() })
    }
}

pub(crate) fn load_body_part<N: WzNode>(frame_node: &N, part_name: &str) -> Option<BodyPart>
where i32: TryFromNode<N>, f32: TryFromNode<N>, String: TryFromNode<N>, bool: TryFromNode<N>, wz_reader::property::Vector2D: TryFromNode<N>
{
    let part_node = frame_node.at_path(part_name).ok()?;
    let origin_node = part_node.try_get("origin")?;
    let origin = origin_node.read_origin(&part_node).ok()?;
    let z_str: String = part_node.at_path("z").ok().and_then(|n| n.into_val().ok())?;
    let image_path = part_node.path();

    let mut map = HashMap::new();
    if let Ok(map_node) = part_node.at_path("map") {
        for (child_name, _) in map_node.children() {
            if let Some(val) = map_node
                .at_path(child_name.as_str()).ok()
                .and_then(|n| {
                    n.into_val::<wz_reader::property::Vector2D>().ok().map(|v| Vector2D(v.0 as f32, -(v.1 as f32)))
                })
            {
                map.insert(child_name.to_string(), val);
            }
        }
    }

    Some(BodyPart {
        image_path,
        origin,
        map,
        z: 0.0,
        part_name: part_name.to_string(),
        slot: Some(z_str),
    })
}

// ── HairBody ──

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HairBody {
    pub frames: Vec<BodyFrame>,
}

impl HairBody {
    pub fn load<N: WzNode>(base: &N, hair_id: u32, action: &str) -> Result<Self, WzError>
    where i32: TryFromNode<N>, f32: TryFromNode<N>, String: TryFromNode<N>, bool: TryFromNode<N>, wz_reader::property::Vector2D: TryFromNode<N>
    {
        let action_path = format!("Character/Hair/{:08}.img/{}", hair_id, action);
        let action_node = base.at_path(&action_path)?;

        let frame_count = action_node.children().len();
        if frame_count == 0 {
            return Ok(HairBody { frames: Vec::new() });
        }

        let mut frames = Vec::with_capacity(frame_count);
        for frame_idx in 0..frame_count as u32 {
            let frame_path = format!("{}/{}", action_path, frame_idx);
            let frame_node = match base.at_path(&frame_path) {
                Ok(n) => n,
                Err(_) => continue,
            };

            let delay: i32 = frame_node.get_or("delay", 100);

            let mut parts = Vec::new();
            for (child_name, _) in frame_node.children() {
                let pn = child_name.as_str();
                if pn == "delay" { continue; }
                if let Some(part) = load_body_part(&frame_node, pn) {
                    parts.push(part);
                }
            }

            if !parts.is_empty() {
                frames.push(BodyFrame { parts, delay: delay.unsigned_abs() });
            }
        }

        Ok(HairBody { frames })
    }
}

// ── FaceFrame / FaceExpression ──

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FaceFrame {
    pub part: BodyPart,
    pub delay: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FaceExpression {
    pub frames: Vec<FaceFrame>,
}

impl FaceExpression {
    pub fn load<N: WzNode>(base: &N, face_id: u32, expression: &str) -> Result<Self, WzError>
    where i32: TryFromNode<N>, f32: TryFromNode<N>, String: TryFromNode<N>, bool: TryFromNode<N>, wz_reader::property::Vector2D: TryFromNode<N>
    {
        let expr_path = format!("Character/Face/{:08}.img/{}", face_id, expression);
        let expr_node = base.at_path(&expr_path)?;

        let child_keys: Vec<String> = expr_node.children().into_iter()
            .map(|(n, _)| n.to_string())
            .collect();

        let mut frames = Vec::new();

        if child_keys.iter().any(|k| k == "face") {
            if let Some(part) = load_body_part(&expr_node, "face") {
                frames.push(FaceFrame { part, delay: 2000 });
            }
        } else if child_keys.iter().any(|k| k.parse::<u32>().is_ok()) {
            for key in &child_keys {
                if let Ok(idx) = key.parse::<u32>() {
                    if let Ok(frame_node) = expr_node.at_path(&idx.to_string()) {
                        if let Ok(delay_node) = frame_node.at_path("delay") {
                            let delay: i32 = delay_node.into_val().unwrap_or(100);
                            if let Some(part) = load_body_part(&frame_node, "face") {
                                frames.push(FaceFrame { part, delay: delay as u32 });
                            }
                        }
                    }
                }
            }
        }

        Ok(FaceExpression { frames })
    }
}
