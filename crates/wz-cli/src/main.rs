use clap::{Parser, Subcommand};
use std::path::PathBuf;
use wz_cli::*;
use wz_reader::WzNodeCast;

const WZ_DIR: &str = "./wz";

#[derive(Parser)]
#[command(name = "wz", about = "Probe MapleStory WZ files", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List children of a path (default: root)
    List {
        /// Path within the WZ tree, e.g. "Map" or "Map/Map/Map1/100000000.img"
        path: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show tree structure to a given depth
    Tree {
        /// Path within the WZ tree
        path: Option<String>,
        /// Maximum depth
        #[arg(short, long, default_value = "2")]
        depth: usize,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show detailed info about a node
    Info {
        /// Path within the WZ tree
        path: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Dump a subtree as JSON
    Dump {
        /// Path within the WZ tree
        #[arg(default_value = "")]
        path: String,
        /// Maximum recursion depth (0 = unlimited)
        #[arg(short, long, default_value = "0")]
        depth: usize,
    },
    /// Search nodes by name substring
    Search {
        /// Substring to search for in node names
        query: String,
        /// Starting path
        #[arg(short, long, default_value = "")]
        path: String,
        /// Maximum results
        #[arg(short, long, default_value = "20")]
        max: usize,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Extract the raw value of a node
    Get {
        /// Path to the node
        path: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Resolve a UOL, _inlink, or _outlink reference to its target node
    Resolve {
        /// Path to a link node
        path: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Export a PNG or sound node to a file in a directory
    Export {
        /// Path to a PNG or sound node
        path: String,
        /// Output directory
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Show the expected structure (child names and types) at a path
    Schema {
        /// Path to a structural node
        path: String,
        /// Recursion depth
        #[arg(short, long, default_value = "2")]
        depth: usize,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let wz_dir = PathBuf::from(WZ_DIR);
    let base = load_base(&wz_dir)?;

    match &cli.command {
        Commands::List { path, json } => {
            let node =
                resolve_path(&base, path.as_deref().unwrap_or("")).ok_or("path not found")?;
            let children = get_children(&node);
            if *json {
                let items: Vec<NodeInfo> = children.iter().map(|(_, n)| get_node_info(n)).collect();
                println!("{}", serde_json::to_string_pretty(&items)?);
            } else {
                for (name, child) in &children {
                    let info = get_node_info(child);
                    let type_abbr = match info.object_type.as_str() {
                        "Directory" => "DIR",
                        "Image" => "IMG",
                        "File" => "WZ",
                        "Property" => "PROP",
                        "Value" => "VAL",
                        "MsFile" => "MS",
                        "MsImage" => "MSI",
                        _ => "???",
                    };
                    let val = info.value.map(|v| format!(" = {}", v)).unwrap_or_default();
                    println!(
                        "  [{:>4}] {} ({} children){val}",
                        type_abbr, name, info.children_count
                    );
                }
                if children.is_empty() {
                    let info = get_node_info(&node);
                    if let Some(v) = info.value {
                        println!("  value: {}", v);
                    }
                }
                println!("{} entries", children.len());
            }
        }
        Commands::Tree { path, depth, json } => {
            let node =
                resolve_path(&base, path.as_deref().unwrap_or("")).ok_or("path not found")?;
            let tree = collect_tree(&node, *depth, 0);
            if *json {
                println!("{}", serde_json::to_string_pretty(&tree)?);
            } else {
                print_tree_node(&tree, 0);
            }
        }
        Commands::Info { path, json } => {
            let node = resolve_path(&base, path).ok_or("path not found")?;
            let info = get_node_info(&node);
            if *json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("Name:    {}", info.name);
                println!("Type:    {}", info.object_type);
                println!("Path:    {}", info.full_path);
                println!(
                    "Value:   {}",
                    info.value
                        .as_ref()
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "(none)".into())
                );
                println!("Children: {}", info.children_count);
                if info.children_count > 0 && info.children_count <= 20 {
                    println!("\nChildren:");
                    for (cname, _) in get_children(&node) {
                        println!("  - {cname}");
                    }
                }
                // Show PNG dimensions if applicable
                if let Some(val) = get_node_value_detail(&node) {
                    if let Some(obj) = val.as_object() {
                        if obj.get("type").and_then(|t| t.as_str()) == Some("png") {
                            let w = obj.get("width").and_then(|v| v.as_u64()).unwrap_or(0);
                            let h = obj.get("height").and_then(|v| v.as_u64()).unwrap_or(0);
                            println!("\nPNG: {}x{}", w, h);
                            if let Some(props) = obj.get("properties").and_then(|p| p.as_object()) {
                                if !props.is_empty() {
                                    println!("Properties:");
                                    for (k, v) in props {
                                        println!(
                                            "  {} = {}",
                                            k,
                                            serde_json::to_string(v).unwrap_or_default()
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Commands::Dump { path, depth } => {
            let node = resolve_path(&base, path).ok_or("path not found")?;
            let max_depth = if *depth == 0 { usize::MAX } else { *depth };
            let tree = collect_tree(&node, max_depth, 0);
            println!("{}", serde_json::to_string_pretty(&tree)?);
        }
        Commands::Get { path, json } => {
            let node = resolve_path(&base, path).ok_or("path not found")?;
            if let Some(val) = get_node_value_detail(&node) {
                if *json {
                    println!("{}", serde_json::to_string_pretty(&val)?);
                } else {
                    print_value(&val);
                }
            } else {
                println!("(no value)");
            }
        }
        Commands::Resolve { path, json } => {
            let node = resolve_path(&base, path).ok_or("path not found")?;
            let guard = node.read().unwrap();
            let link_info = {
                if guard.try_as_uol().is_some() {
                    let p = guard
                        .try_as_uol()
                        .and_then(|s| s.get_string().ok())
                        .unwrap_or_default();
                    ("UOL", p)
                } else if let Some(inlink) = guard.children.get("_inlink") {
                    let p = inlink
                        .read()
                        .ok()
                        .and_then(|g| g.try_as_string().and_then(|s| s.get_string().ok()))
                        .unwrap_or_default();
                    ("_inlink", p)
                } else if let Some(outlink) = guard.children.get("_outlink") {
                    let p = outlink
                        .read()
                        .ok()
                        .and_then(|g| g.try_as_string().and_then(|s| s.get_string().ok()))
                        .unwrap_or_default();
                    ("_outlink", p)
                } else {
                    ("", String::new())
                }
            };
            drop(guard);
            if *json {
                let result = match link_info {
                    ("", _) => {
                        serde_json::json!({"link_type": null, "link_path": null, "target": null})
                    }
                    (link_type, link_path) => {
                        serde_json::json!({
                            "link_type": link_type,
                            "link_path": link_path,
                            "target": resolve_link_target(&node).as_ref().map(|t| get_node_info(t)),
                        })
                    }
                };
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                match link_info {
                    ("", _) => println!("not a link node"),
                    (link_type, link_path) => {
                        let target = resolve_link_target(&node);
                        println!("Link type: {link_type}");
                        println!("Link path: {link_path}");
                        if let Some(t) = target {
                            let info = get_node_info(&t);
                            println!("Target: {} [{}]", info.full_path, info.object_type);
                        } else {
                            println!("Target: (unresolved)");
                        }
                    }
                }
            }
        }
        Commands::Export { path, output } => {
            let node = resolve_path(&base, path).ok_or("path not found")?;
            export_recursive(&node, output)?;
        }
        Commands::Schema { path, depth, json } => {
            let node = resolve_path(&base, path).ok_or("path not found")?;
            let schema = schema_tree(&node, *depth);
            if *json {
                println!("{}", serde_json::to_string_pretty(&schema)?);
            } else {
                print_schema(&schema, 0);
            }
        }
        Commands::Search {
            query,
            path,
            max,
            json,
        } => {
            let node = resolve_path(&base, path).ok_or("path not found")?;
            let results = std::sync::Mutex::new(Vec::new());
            walk_nodes(&node, true, |n| {
                let guard = n.read().unwrap();
                if guard.name.as_str().contains(query) {
                    let mut list = results.lock().unwrap();
                    if list.len() < *max {
                        list.push(get_node_info(n));
                    }
                }
            });
            let results = results.into_inner().unwrap();
            if *json {
                println!("{}", serde_json::to_string_pretty(&results)?);
            } else {
                for r in &results {
                    let val = r
                        .value
                        .as_ref()
                        .map(|v| format!(" = {v}"))
                        .unwrap_or_default();
                    println!("  {}  [{}]{}", r.full_path, r.object_type, val);
                }
                println!("{} matches", results.len());
            }
        }
    }
    Ok(())
}

fn print_value(val: &serde_json::Value) {
    match val {
        serde_json::Value::String(s) => println!("{s}"),
        serde_json::Value::Number(n) => println!("{n}"),
        serde_json::Value::Bool(b) => println!("{b}"),
        serde_json::Value::Array(a) => println!("{:?}", a),
        serde_json::Value::Object(o) => {
            if o.get("type").and_then(|t| t.as_str()) == Some("png") {
                let w = o.get("width").and_then(|v| v.as_u64()).unwrap_or(0);
                let h = o.get("height").and_then(|v| v.as_u64()).unwrap_or(0);
                println!("PNG {w}x{h}");
                if let Some(props) = o.get("properties").and_then(|p| p.as_object()) {
                    for (k, v) in props {
                        print!("  {k}: ");
                        print_value_inline(v);
                    }
                }
            } else if o.get("type").and_then(|t| t.as_str()) == Some("sound") {
                let dur = o.get("duration").and_then(|v| v.as_u64()).unwrap_or(0);
                let fmt = o.get("format").and_then(|v| v.as_str()).unwrap_or("?");
                println!("Sound [{fmt}] {dur}ms");
            } else if o.get("type").and_then(|t| t.as_str()) == Some("lua") {
                println!("<Lua script>");
            } else {
                println!("{}", serde_json::to_string_pretty(val).unwrap_or_default());
            }
        }
        serde_json::Value::Null => println!("null"),
    }
}

fn print_value_inline(val: &serde_json::Value) {
    match val {
        serde_json::Value::String(s) => println!("\"{s}\""),
        serde_json::Value::Number(n) => println!("{n}"),
        serde_json::Value::Bool(b) => println!("{b}"),
        serde_json::Value::Array(a) => println!("{a:?}"),
        _ => println!("{}", serde_json::to_string(val).unwrap_or_default()),
    }
}

fn print_schema(val: &serde_json::Value, indent: usize) {
    if let Some(obj) = val.as_object() {
        if let Some(path) = obj.get("path").and_then(|v| v.as_str()) {
            println!("{}{}/", "  ".repeat(indent), path);
        }
        if let Some(schema) = obj.get("schema") {
            print_schema_node(schema, indent + 1);
        }
    }
}

fn print_schema_node(val: &serde_json::Value, indent: usize) {
    let prefix = "  ".repeat(indent);
    if let Some(obj) = val.as_object() {
        let typ = obj.get("type").and_then(|v| v.as_str()).unwrap_or("?");
        let vt = obj.get("value_type").and_then(|v| v.as_str());
        if let Some(vt) = vt {
            print!("{prefix}[{typ}] ({vt})");
        } else {
            print!("{prefix}[{typ}]");
        }
        if let Some(ch) = obj.get("children") {
            match ch {
                serde_json::Value::Object(children) => {
                    if children.is_empty() {
                        println!();
                    } else {
                        println!();
                        for (name, child) in children {
                            print!("{}  {name}: ", prefix);
                            print_schema_node(child, indent + 1);
                        }
                    }
                }
                serde_json::Value::Number(n) => println!(" ({n} children)"),
                _ => println!(),
            }
        } else {
            println!();
        }
    }
}

fn print_tree_node(node: &TreeNode, indent: usize) {
    let prefix = "  ".repeat(indent);
    let info = &node.info;
    let val = info
        .value
        .as_ref()
        .map(|v| format!(" = {v}"))
        .unwrap_or_default();
    let child_info = if !node.children.is_empty() {
        format!(" ({})", info.children_count)
    } else {
        String::new()
    };
    println!("{prefix}- {}{}{}", info.name, child_info, val);
    for child in &node.children {
        print_tree_node(child, indent + 1);
    }
}
