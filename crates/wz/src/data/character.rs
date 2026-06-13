use std::collections::HashMap;
use crate::error::WzError;
use crate::node::Node;
use crate::vector2d::Vector2D;
use crate::data::common::{FrameData, SpriteLayerData, PartSource};

#[derive(Debug, Clone)]
pub struct CharacterData {
    pub body: HashMap<String, Vec<FrameData>>,
    pub head: HashMap<String, Vec<FrameData>>,
    pub hair: HashMap<String, Vec<FrameData>>,
    pub face_expressions: HashMap<String, Vec<FrameData>>,
}

impl CharacterData {
    pub(crate) fn load(base: &Node, skin_suffix: u32, hair_id: u32, face_id: u32) -> Result<Self, WzError> {
        let body_path = format!("Character/0000{:04}.img", skin_suffix);
        let head_path = format!("Character/0001{:04}.img", skin_suffix);

        let body_root = base.at_path(&body_path)?;
        let head_root = base.at_path(&head_path).ok();

        let body = load_actions(&body_root, &body_path, PartSource::Body, base)?;
        let head = head_root.as_ref()
            .map(|h| load_actions(h, &head_path, PartSource::Head, base))
            .transpose()?
            .unwrap_or_default();
        let hair = load_hair(base, hair_id)?;
        let face_expressions = load_face_expressions(base, face_id)?;

        Ok(CharacterData { body, head, hair, face_expressions })
    }
}

fn load_actions(root: &Node, root_path: &str, source: PartSource, base: &Node) -> Result<HashMap<String, Vec<FrameData>>, WzError> {
    let mut actions = HashMap::new();

    for (action_name, _) in root.children() {
        let action_name = String::from(action_name);
        if action_name == "info" { continue; }

        let action_node = match root.at_path(&action_name) {
            Ok(n) => n,
            Err(_) => continue,
        };

        let frame_count = action_node.children().len();
        if frame_count == 0 { continue; }

        let mut frames = Vec::new();
        let action_path = format!("{}/{}", root_path, action_name);

        for frame_idx in 0..frame_count as u32 {
            if let Some(frame) = load_single_frame(base, &action_path, &action_name, frame_idx, source) {
                frames.push(frame);
            }
        }

        if !frames.is_empty() {
            actions.insert(action_name, frames);
        }
    }

    Ok(actions)
}

fn load_single_frame(base: &Node, action_path: &str, _action_name: &str, frame_idx: u32, source: PartSource) -> Option<FrameData> {
    let frame_path = format!("{}/{}", action_path, frame_idx);
    let frame_node = base.at_path(&frame_path).ok()?;

    let delay: i32 = frame_node
        .at_path("delay").ok()
        .and_then(|n| n.try_into().ok())
        .unwrap_or(100);

    // Check if this is a frame-reference action
    if let (Ok(action_node), Ok(frame_node_val)) = (
        frame_node.at_path("action"),
        frame_node.at_path("frame"),
    ) {
        let ref_action: String = action_node.try_into().ok()?;
        let ref_frame: i32 = frame_node_val.try_into().ok()?;
        let ref_body_action_path = action_path
            .rsplit_once('/')
            .map(|(parent, _)| format!("{}/{}", parent, ref_action))
            .unwrap_or_else(|| format!("{}/{}", action_path, ref_action));

        let mut frame = load_single_frame(base, &ref_body_action_path, &ref_action, ref_frame as u32, source)?;
        frame.delay = delay.unsigned_abs();
        return Some(frame);
    }

    let mut parts = Vec::new();

    for (child_name, _) in frame_node.children() {
        if child_name.as_str() == "delay" {
            continue;
        }
        if let Some(layer) = load_part(&frame_node, child_name.as_str(), source) {
            parts.push(layer);
        }
    }

    Some(FrameData { parts, delay: delay.unsigned_abs() })
}

fn load_part(frame_node: &Node, part_name: &str, source: PartSource) -> Option<SpriteLayerData> {
    let part_node = frame_node.at_path(part_name).ok()?;
    let origin_node = part_node.try_get("origin")?;
    let origin = origin_node.read_origin(&part_node).ok()?;
    let z_str: String = part_node.at_path("z").ok().and_then(|n| n.try_into().ok())?;
    let image_path = part_node.path();

    let mut map = HashMap::new();
    if let Ok(map_node) = part_node.at_path("map") {
        for (child_name, _) in map_node.children() {
            if let Some(val) = map_node
                .at_path(child_name.as_str()).ok()
                .and_then(|n| {
                    let v: Result<wz_reader::property::Vector2D, _> = n.try_into();
                    v.ok().map(|v| Vector2D(v.0 as f32, -(v.1 as f32)))
                })
            {
                map.insert(child_name.to_string(), val);
            }
        }
    }

    Some(SpriteLayerData {
        image_path,
        origin,
        map,
        z: 0.0, // zmap depth applied by consumer
        layer_name: part_name.to_string(),
        slot: Some(z_str),
        source,
    })
}

fn load_hair(base: &Node, hair_id: u32) -> Result<HashMap<String, Vec<FrameData>>, WzError> {
    let hair_path = format!("Character/Hair/{:08}.img", hair_id);
    let hair_root = match base.at_path(&hair_path) {
        Ok(n) => n,
        Err(_) => return Ok(HashMap::new()),
    };

    let mut actions = HashMap::new();
    for (action_name, _) in hair_root.children() {
        let action_name = String::from(action_name);
        if action_name == "info" { continue; }

        let action_node = match hair_root.at_path(&action_name) {
            Ok(n) => n,
            Err(_) => continue,
        };

        let frame_count = action_node.children().len();
        if frame_count == 0 { continue; }

        let mut frames = Vec::new();
        for frame_idx in 0..frame_count as u32 {
            let frame_path = format!("{}/{}/{}", hair_path, action_name, frame_idx);
            let frame_node = match base.at_path(&frame_path) {
                Ok(n) => n,
                Err(_) => continue,
            };

            let delay: i32 = frame_node.at_path("delay").ok()
                .and_then(|n| n.try_into().ok())
                .unwrap_or(100);

            let mut parts = Vec::new();
            for (part_name, _) in frame_node.children() {
                let pn = part_name.as_str();
                if pn == "delay" { continue; }
                if let Some(layer) = load_part(&frame_node, pn, PartSource::Hair) {
                    parts.push(layer);
                }
            }

            if !parts.is_empty() {
                frames.push(FrameData { parts, delay: delay.unsigned_abs() });
            }
        }

        if !frames.is_empty() {
            actions.insert(action_name, frames);
        }
    }

    Ok(actions)
}

fn load_face_expressions(base: &Node, face_id: u32) -> Result<HashMap<String, Vec<FrameData>>, WzError> {
    let face_path = format!("Character/Face/{:08}.img", face_id);
    let face_root = match base.at_path(&face_path) {
        Ok(n) => n,
        Err(_) => return Ok(HashMap::new()),
    };

    let mut result = HashMap::new();

    for (expr_name, _) in face_root.children() {
        let expr_name = String::from(expr_name);
        if expr_name == "info" { continue; }

        let expr_node = match face_root.at_path(&expr_name) {
            Ok(n) => n,
            Err(_) => continue,
        };

        let child_keys: Vec<String> = expr_node.children().into_iter()
            .map(|(n, _)| n.to_string())
            .collect();

        let mut frames = Vec::new();

        if child_keys.iter().any(|k| k == "face") {
            if let Some(layer) = load_part(&expr_node, "face", PartSource::Face) {
                frames.push(FrameData { parts: vec![layer], delay: 2000 });
            }
        } else if child_keys.iter().any(|k| k.parse::<u32>().is_ok()) {
            for key in &child_keys {
                if let Ok(idx) = key.parse::<u32>() {
                    if let Ok(frame_node) = expr_node.at_path(&idx.to_string()) {
                        if let Ok(delay_node) = frame_node.at_path("delay") {
                            let delay: Result<i32, _> = delay_node.try_into();
                            if let Ok(delay) = delay {
                                if let Some(layer) = load_part(&frame_node, "face", PartSource::Face) {
                                    frames.push(FrameData { parts: vec![layer], delay: delay as u32 });
                                }
                            }
                        }
                    }
                }
            }
        }

        if !frames.is_empty() {
            result.insert(expr_name, frames);
        }
    }

    Ok(result)
}
