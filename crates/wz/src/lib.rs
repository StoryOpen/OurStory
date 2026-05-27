use serde::Serialize;
use std::path::Path;
use wz_reader::node::WzNodeArc;
use wz_reader::util::walk::walk_node;
use wz_reader::WzNodeCast;

pub fn load_base(wz_dir: &Path) -> Result<WzNodeArc, Box<dyn std::error::Error>> {
    let base_path = wz_dir.join("Base.wz");
    let node = wz_reader::util::resolve_base(&base_path, None)?;
    Ok(node)
}

pub fn parse_node(node: &WzNodeArc) -> Result<(), wz_reader::node::Error> {
    wz_reader::util::node_util::parse_node(node)
}

pub fn resolve_path(base: &WzNodeArc, path: &str) -> Option<WzNodeArc> {
    if path.is_empty() {
        return Some(base.clone());
    }
    let segments: Vec<&str> = path.split('/').collect();
    let mut current = base.clone();
    for seg in &segments {
        let child = {
            let guard = current.read().ok()?;
            guard.at(seg)?
        };
        if seg.ends_with(".img") {
            parse_node(&child).ok()?;
        }
        current = child;
    }
    Some(current)
}

pub fn ensure_parsed(node: &WzNodeArc) {
    let guard = node.read().unwrap();
    let needs_parse = matches!(&guard.object_type, wz_reader::WzObjectType::Image(_))
        && guard.children.is_empty();
    drop(guard);
    if needs_parse {
        parse_node(node).ok();
    }
}

pub fn get_children(node: &WzNodeArc) -> Vec<(String, WzNodeArc)> {
    ensure_parsed(node);
    let guard = node.read().unwrap();
    guard
        .children
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

pub fn get_node_info(node: &WzNodeArc) -> NodeInfo {
    let guard = node.read().unwrap();
    let type_str = object_type_name(&guard.object_type);
    let value = extract_value(&guard);
    let children_count = guard.children.len();
    NodeInfo {
        name: guard.name.to_string(),
        object_type: type_str,
        value,
        children_count,
        full_path: guard.get_full_path(),
    }
}

pub fn collect_tree(
    node: &WzNodeArc,
    max_depth: usize,
    current_depth: usize,
) -> TreeNode {
    let info = get_node_info(node);
    let mut children = Vec::new();
    if current_depth < max_depth {
        for (_, child) in get_children(node) {
            children.push(collect_tree(&child, max_depth, current_depth + 1));
        }
    }
    TreeNode { info, children }
}

#[derive(Serialize, Debug, Clone)]
pub struct NodeInfo {
    pub name: String,
    pub object_type: String,
    pub value: Option<serde_json::Value>,
    pub children_count: usize,
    pub full_path: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct TreeNode {
    pub info: NodeInfo,
    pub children: Vec<TreeNode>,
}

pub fn walk_nodes<F>(node: &WzNodeArc, force_parse: bool, f: F)
where
    F: Fn(&WzNodeArc),
{
    walk_node(node, force_parse, &f);
}

fn object_type_name(obj: &wz_reader::WzObjectType) -> String {
    match obj {
        wz_reader::WzObjectType::File(_) => "File".into(),
        wz_reader::WzObjectType::MsFile(_) => "MsFile".into(),
        wz_reader::WzObjectType::Image(_) => "Image".into(),
        wz_reader::WzObjectType::MsImage(_) => "MsImage".into(),
        wz_reader::WzObjectType::Directory(_) => "Directory".into(),
        wz_reader::WzObjectType::Property(_) => "Property".into(),
        wz_reader::WzObjectType::Value(_) => "Value".into(),
    }
}

fn extract_value(node: &wz_reader::node::WzNode) -> Option<serde_json::Value> {
    if let Some(v) = node.try_as_int() {
        return Some(serde_json::json!(v));
    }
    if let Some(v) = node.try_as_short() {
        return Some(serde_json::json!(v));
    }
    if let Some(v) = node.try_as_long() {
        return Some(serde_json::json!(v));
    }
    if let Some(v) = node.try_as_float() {
        return Some(serde_json::json!(v));
    }
    if let Some(v) = node.try_as_double() {
        return Some(serde_json::json!(v));
    }
    if let Some(v) = node.try_as_string() {
        if let Ok(s) = v.get_string() {
            return Some(serde_json::json!(s));
        }
    }
    if let Some(v) = node.try_as_vector2d() {
        return Some(serde_json::json!({ "x": v.0, "y": v.1 }));
    }
    if node.try_as_png().is_some() {
        return Some(serde_json::json!("<PNG image>"));
    }
    if node.try_as_sound().is_some() {
        return Some(serde_json::json!("<Sound>"));
    }
    if node.try_as_lua().is_some() {
        return Some(serde_json::json!("<Lua script>"));
    }
    if node.try_as_video().is_some() {
        return Some(serde_json::json!("<Video>"));
    }
    None
}
