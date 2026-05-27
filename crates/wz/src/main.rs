use clap::{Parser, Subcommand};
use std::path::PathBuf;
use wz::*;

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
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let wz_dir = PathBuf::from(WZ_DIR);
    let base = load_base(&wz_dir)?;

    match &cli.command {
        Commands::List { path, json } => {
            let node = resolve_path(&base, path.as_deref().unwrap_or(""))
                .ok_or("path not found")?;
            let children = get_children(&node);
            if *json {
                let items: Vec<NodeInfo> = children
                    .iter()
                    .map(|(_, n)| get_node_info(n))
                    .collect();
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
                    let val = info
                        .value
                        .map(|v| format!(" = {}", v))
                        .unwrap_or_default();
                    println!("  [{:>4}] {} ({} children){val}", type_abbr, name, info.children_count);
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
        Commands::Tree {
            path,
            depth,
            json,
        } => {
            let node = resolve_path(&base, path.as_deref().unwrap_or(""))
                .ok_or("path not found")?;
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
            }
        }
        Commands::Dump { path, depth } => {
            let node = resolve_path(&base, path).ok_or("path not found")?;
            let max_depth = if *depth == 0 { usize::MAX } else { *depth };
            let tree = collect_tree(&node, max_depth, 0);
            println!("{}", serde_json::to_string_pretty(&tree)?);
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
