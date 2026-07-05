use log::warn;
use std::collections::{BTreeMap, HashMap};
use crate::error::WzError;
use crate::node_trait::{WzNode, TryFromNode};
use crate::vector2d::Vector2D;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NpcData {
    pub id: i32,
    pub name: String,
    pub actions: HashMap<String, NpcAction>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NpcAction {
    pub frames: Vec<NpcFrame>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NpcFrame {
    pub delay: u32,
    pub image_path: String,
    pub origin: Vector2D,
}

impl NpcData {
    pub(crate) fn load<N: WzNode>(base: &N, id: i32) -> Result<Self, WzError>
    where i32: TryFromNode<N>, f32: TryFromNode<N>, String: TryFromNode<N>, bool: TryFromNode<N>
{
        let wz_path = format!("Npc/{:07}.img", id);
        let npc_node = base.at_path(&wz_path)?;

        let name = base.at_path(&format!("String/Npc.img/{id}/name"))
            .ok()
            .and_then(|n| -> Option<String> { n.into_val().ok() })
            .unwrap_or_else(|| {
                warn!("Npc {id}: name not found, using default");
                String::new()
            });

        let mut actions = HashMap::new();
        for (action_name, action_node) in npc_node.children() {
            let name = action_name.to_string();
            if name == "info" { continue; }
            if !action_node.try_get("0").is_some() { continue; }

            let mut frame_map: BTreeMap<u32, NpcFrame> = BTreeMap::new();
            for (frame_key, frame_node) in action_node.children() {
                let frame_index = match frame_key.to_string().parse::<u32>() {
                    Ok(i) => i,
                    Err(_) => continue,
                };

                let delay = frame_node.get_or("delay", 100 );

                let image_path = frame_node.path();
                let origin = frame_node
                    .try_get("origin")
                    .and_then(|n| n.read_origin(&frame_node).ok())
                    .unwrap_or_else(|| {
                        warn!("Npc {id}: frame '{}' missing origin, using ZERO", frame_node.path());
                        Vector2D::ZERO
                    });

                frame_map.insert(frame_index, NpcFrame { delay: delay.unsigned_abs(), image_path, origin });
            }

            let frames: Vec<NpcFrame> = frame_map.into_values().collect();
            if !frames.is_empty() {
                actions.insert(name, NpcAction { frames });
            }
        }

        Ok(NpcData { id, name, actions })
    }
}
