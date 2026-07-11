use std::path::PathBuf;
use std::sync::OnceLock;

use wz_reader::util::node_util::parse_node;
use wz_reader::WzNodeArc;

use crate::error::WzError;
use crate::node::Node;

/// Errors from the on-disk WZ data source.
#[derive(Debug, thiserror::Error)]
pub enum WzSourceError {
    #[error("wz source not initialized (Base.wz failed to load)")]
    NotInitialized,
    #[error("failed to load Base.wz: {0}")]
    Load(String),
    #[error(transparent)]
    Node(#[from] WzError),
}

/// Lazily-loaded root of the WZ tree, resolved from a `Base.wz` directory.
pub struct WzSource {
    root: Option<Node>,
}

static ROOT: OnceLock<Option<Node>> = OnceLock::new();

fn root_node() -> Option<Node> {
    let cell = ROOT.get_or_init(load_root);
    cell.clone()
}

/// Load `Base.wz` (and its cross-file links) from `WZ_DIR` or `./wz`.
fn load_root() -> Option<Node> {
    let dir = std::env::var("WZ_DIR").unwrap_or_else(|_| "./wz".to_string());
    let base_path = PathBuf::from(dir).join("Base.wz");
    match wz_reader::util::resolve_base(&base_path, None) {
        Ok(node) => Some(node.into()),
        Err(e) => {
            log::error!("failed to load wz base from {}: {e}", base_path.display());
            None
        }
    }
}

impl WzSource {
    /// Resolve a WZ node path (e.g. `Map/Map/Map1/100010000.img`).
    pub fn node(&self, path: &str) -> Result<Node, WzSourceError> {
        let root = self.root.as_ref().ok_or(WzSourceError::NotInitialized)?;
        let resolved = root.at_path(path)?;
        parse_subtree(&resolved.wz_node);
        Ok(resolved)
    }
}

/// Recursively parse a node and all of its descendants.
fn parse_subtree(node: &WzNodeArc) {
    let _ = parse_node(node);
    let children: Vec<WzNodeArc> = node
        .read()
        .map(|g| g.children.values().cloned().collect())
        .unwrap_or_default();
    for child in &children {
        parse_subtree(child);
    }
}

/// Get the process-wide WZ source. Loads `Base.wz` on first use.
pub fn default_source() -> WzSource {
    WzSource { root: root_node() }
}
