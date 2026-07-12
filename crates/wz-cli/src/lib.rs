use serde::Serialize;
use std::path::Path;
use wz_reader::WzNodeCast;
use wz_reader::node::WzNodeArc;
use wz_reader::util::walk::walk_node;

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

pub fn collect_tree(node: &WzNodeArc, max_depth: usize, current_depth: usize) -> TreeNode {
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

pub fn value_type_name(node: &WzNodeArc) -> String {
    let guard = node.read().unwrap();
    if guard.try_as_int().is_some() {
        return "int".into();
    }
    if guard.try_as_short().is_some() {
        return "short".into();
    }
    if guard.try_as_long().is_some() {
        return "long".into();
    }
    if guard.try_as_float().is_some() {
        return "float".into();
    }
    if guard.try_as_double().is_some() {
        return "double".into();
    }
    if guard.try_as_string().is_some() {
        return "string".into();
    }
    if guard.try_as_vector2d().is_some() {
        return "vector".into();
    }
    if guard.try_as_png().is_some() {
        return "png".into();
    }
    if guard.try_as_sound().is_some() {
        return "sound".into();
    }
    if guard.try_as_lua().is_some() {
        return "lua".into();
    }
    if guard.try_as_video().is_some() {
        return "video".into();
    }
    if guard.try_as_uol().is_some() {
        return "uol".into();
    }
    "unknown".into()
}

pub fn get_node_value_detail(node: &WzNodeArc) -> Option<serde_json::Value> {
    let guard = node.read().ok()?;
    if let Some(v) = guard.try_as_int() {
        return Some(serde_json::json!(v));
    }
    if let Some(v) = guard.try_as_short() {
        return Some(serde_json::json!(v));
    }
    if let Some(v) = guard.try_as_long() {
        return Some(serde_json::json!(v));
    }
    if let Some(v) = guard.try_as_float() {
        return Some(serde_json::json!(v));
    }
    if let Some(v) = guard.try_as_double() {
        return Some(serde_json::json!(v));
    }
    if let Some(v) = guard.try_as_string() {
        if let Ok(s) = v.get_string() {
            return Some(serde_json::json!(s));
        }
    }
    if let Some(v) = guard.try_as_vector2d() {
        return Some(serde_json::json!({"x": v.0, "y": v.1}));
    }
    let png_meta = guard.try_as_png().map(|p| (p.width, p.height));
    drop(guard);
    if let Some((w, h)) = png_meta {
        let mut props = serde_json::Map::new();
        for (name, child) in get_children(node) {
            if let Some(val) = get_node_value_detail(&child) {
                props.insert(name, val);
            }
        }
        return Some(serde_json::json!({
            "type": "png",
            "width": w,
            "height": h,
            "properties": props,
        }));
    }
    let guard = node.read().ok()?;
    if let Some(snd) = guard.try_as_sound() {
        return Some(serde_json::json!({
            "type": "sound",
            "duration": snd.duration,
            "format": format!("{:?}", snd.sound_type),
        }));
    }
    if let Some(lua) = guard.try_as_lua() {
        if let Ok(script) = lua.extract_lua() {
            return Some(serde_json::json!({"type": "lua", "content": script}));
        }
    }
    if guard.try_as_video().is_some() {
        return Some(serde_json::json!({"type": "video"}));
    }
    if guard.try_as_uol().is_some() {
        drop(guard);
        if let Some(target) = resolve_link_target(node) {
            return get_node_value_detail(&target);
        }
    }
    None
}

pub fn resolve_link_target(node: &WzNodeArc) -> Option<WzNodeArc> {
    let guard = node.read().ok()?;
    if let Some(uol_str) = guard.try_as_uol() {
        let path = uol_str.get_string().ok()?;
        drop(guard);
        let parent = node.read().ok()?.parent.upgrade()?;
        let target = parent.read().ok()?.at_path_relative(&path)?;
        if target.read().ok().map_or(false, |g| {
            matches!(&g.object_type, wz_reader::WzObjectType::Image(_))
        }) {
            parse_node(&target).ok()?;
        }
        return Some(target);
    }
    if let Some(inlink) = guard.children.get("_inlink") {
        let path = inlink.read().ok()?.try_as_string()?.get_string().ok()?;
        drop(guard);
        return wz_reader::util::node_util::resolve_inlink(&path, node);
    }
    if let Some(outlink) = guard.children.get("_outlink") {
        let path = outlink.read().ok()?.try_as_string()?.get_string().ok()?;
        drop(guard);
        return wz_reader::util::node_util::resolve_outlink(&path, node, true);
    }
    None
}

pub fn export_node(node: &WzNodeArc, output_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(output_dir)?;
    let name = node
        .read()
        .map_err(|e| format!("lock error: {e}"))?
        .name
        .to_string();
    if let Ok(img) = wz_reader::property::get_image(node) {
        let path = output_dir.join(format!("{name}.png"));
        img.save(&path)?;
        println!("Exported {name}.png");
        return Ok(());
    }
    {
        let guard = node.read().map_err(|e| format!("lock error: {e}"))?;
        if let Some(sound) = guard.try_as_sound() {
            let path = output_dir.join(&name);
            sound.save(path)?;
            println!("Exported {name}");
            return Ok(());
        }
    }
    if let Some(target) = resolve_link_target(node) {
        return export_node(&target, output_dir);
    }
    Err("node is not a PNG or sound, and has no resolvable link".into())
}

/// Recursively export a node and all of its descendant PNG/sound leaves,
/// mirroring the WZ tree as a directory structure under `output_dir`.
/// A leaf PNG/sound is exported directly into `output_dir`; a container node
/// recurses into a per-child subdirectory.
pub fn export_recursive(
    node: &WzNodeArc,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if wz_reader::property::get_image(node).is_ok() {
        return export_node(node, output_dir);
    }
    {
        let guard = node.read().map_err(|e| format!("lock error: {e}"))?;
        if guard.try_as_sound().is_some() {
            return export_node(node, output_dir);
        }
    }
    if let Some(target) = resolve_link_target(node) {
        return export_recursive(&target, output_dir);
    }
    std::fs::create_dir_all(output_dir)?;
    for (name, child) in get_children(node) {
        let child_dir = output_dir.join(&name);
        export_recursive(&child, &child_dir)?;
    }
    Ok(())
}

pub fn schema_tree(node: &WzNodeArc, depth: usize) -> serde_json::Value {
    fn build(node: &WzNodeArc, depth: usize) -> serde_json::Value {
        let guard = node.read().unwrap();
        let mut map = serde_json::Map::new();
        map.insert(
            "type".into(),
            serde_json::json!(object_type_name(&guard.object_type)),
        );
        let vt = {
            if guard.try_as_int().is_some() {
                Some("int")
            } else if guard.try_as_short().is_some() {
                Some("short")
            } else if guard.try_as_long().is_some() {
                Some("long")
            } else if guard.try_as_float().is_some() {
                Some("float")
            } else if guard.try_as_double().is_some() {
                Some("double")
            } else if guard.try_as_string().is_some() {
                Some("string")
            } else if guard.try_as_vector2d().is_some() {
                Some("vector")
            } else if guard.try_as_png().is_some() {
                Some("png")
            } else if guard.try_as_sound().is_some() {
                Some("sound")
            } else {
                None
            }
        };
        if let Some(vt) = vt {
            map.insert("value_type".into(), serde_json::json!(vt));
        }
        if depth > 0 {
            let children_count = guard.children.len();
            drop(guard);
            let children: serde_json::Map<_, _> = get_children(node)
                .into_iter()
                .map(|(name, child)| (name, build(&child, depth - 1)))
                .collect();
            if !children.is_empty() {
                map.insert("children".into(), serde_json::Value::Object(children));
            } else {
                map.insert("children".into(), serde_json::json!(children_count));
            }
        }
        serde_json::Value::Object(map)
    }
    serde_json::json!({
        "path": get_node_info(node).full_path,
        "schema": build(node, depth)
    })
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
