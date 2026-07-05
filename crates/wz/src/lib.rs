pub mod error;
pub mod vector2d;
pub mod node;
pub mod data;
pub mod source;
pub mod node_trait;
pub mod json_node;
pub use json_node::JsonNode;

pub use error::{WzError, NodeError};
pub use vector2d::Vector2D;
pub use node::{Node, NodeName, NodePayload};
pub use node_trait::{WzNode, TryFromNode};
pub use data::WzData;
pub use data::{PortalFrameData, MapBundle, MobBundle, NpcBundle, ImageBundle};
pub use data::common::{Foothold, AnimFrame};
pub use data::map::*;
pub use data::mob::*;
pub use data::npc::*;
pub use data::character::*;
pub use data::equip::*;
pub use data::skill::*;
pub use data::quest::*;
pub use data::physics::*;

use std::sync::OnceLock;

static WZ_BASE: OnceLock<Node> = OnceLock::new();

fn resolve_base_node() -> &'static Node {
    WZ_BASE.get_or_init(|| {
        let path = std::env::var("WZ_PATH").unwrap_or_else(|_| "./wz/Base.wz".to_string());
        let wz_node = wz_reader::util::resolve_base(&path, None).expect("resolve_base failed");
        Node::from(wz_node)
    })
}

pub fn get_cached_base() -> &'static Node {
    resolve_base_node()
}

pub fn resolve_base() -> Result<(), WzError> {
    resolve_base_node();
    Ok(())
}

pub(crate) fn set_base_node(node: Node) {
    let _ = WZ_BASE.set(node);
}
