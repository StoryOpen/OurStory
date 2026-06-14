use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;
use tracing::info;
use wz::Node;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEntry {
    pub path: String,
    pub name: String,
    pub text: String,
    pub category: String,
    pub wz_file: String,
    pub thumbnail_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub results: Vec<SearchEntry>,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchIndex {
    pub entries: Vec<SearchEntry>,
}

impl SearchIndex {
    pub fn build() -> Self {
        let base = wz::get_cached_base();
        let mut entries = Vec::new();
        let mut indexed_ids: HashSet<(String, String)> = HashSet::new();

        let map_names = build_map_name_mapping(&base);
        info!("Map names from String.wz: {}", map_names.len());

        let c1 = build_string_index(&base, &mut entries, &mut indexed_ids);
        info!("String.wz: {c1} entries");

        let c2 = build_map_index(&base, &map_names, &mut entries, &mut indexed_ids);
        info!("Map.wz: {c2} entries");

        let skill_map = build_skill_mapping(&base);
        info!("Skills mapped: {}", skill_map.len());

        if let Ok(skill_img) = base.at_path("String/Skill.img") {
            for (_, child) in skill_img.children() {
                let id = child.name();
                let text = collect_text(&child);
                if text.is_empty() {
                    continue;
                }
                let data_path = match skill_map.get(&id) {
                    Some(p) => p.clone(),
                    None => format!("String/Skill.img/{id}"),
                };
                let thumb = skill_map
                    .get(&id)
                    .map(|p| format!("{p}/icon"));
                entries.push(SearchEntry {
                    path: data_path,
                    name: id.clone(),
                    text,
                    category: "Skill".into(),
                    wz_file: "Skill.wz".into(),
                    thumbnail_path: thumb,
                });
                indexed_ids.insert(("Skill".into(), id));
            }
        }

        let c3 = build_filename_index(&base, &mut entries, &indexed_ids);
        info!("Filenames: {c3} entries");

        resolve_thumbnails(&base, &mut entries);
        let valid: usize = entries.iter().filter(|e| e.thumbnail_path.is_some()).count();
        info!("Resolved thumbnails: {valid}/{} valid", entries.len());

        info!("Total index entries: {}", entries.len());
        SearchIndex { entries }
    }

    pub fn search(&self, query: &str, category: Option<&str>) -> SearchResult {
        let terms: Vec<&str> = query.split_whitespace().collect();
        if terms.is_empty() {
            return SearchResult {
                results: vec![],
                categories: vec!["All".into()],
            };
        }

        let terms_lower: Vec<String> = terms.iter().map(|t| t.to_lowercase()).collect();

        let mut scored: Vec<(i32, &SearchEntry)> = self
            .entries
            .iter()
            .filter_map(|e| {
                if let Some(cat) = category {
                    if cat != "All" && e.category != cat {
                        return None;
                    }
                }
                let mut score = 0i32;
                let name_lower = e.name.to_lowercase();
                let text_lower = e.text.to_lowercase();
                for term in &terms_lower {
                    if !text_lower.contains(term) && !name_lower.contains(term) {
                        return None;
                    }
                    if name_lower == *term {
                        score += 10_000;
                    } else if name_lower.starts_with(term) {
                        score += 5_000;
                    } else if name_lower.contains(term) {
                        score += 1_000;
                    }
                    // Text match: first line (name) ranks highest with
                    // position bonus; other lines get lower score.
                    // Shorter first line → more focused result.
                    let first_line = e.text.lines().next().unwrap_or("");
                    let first_lower = first_line.to_lowercase();
                    if first_lower.contains(term) {
                        let pos = first_lower.find(term).unwrap_or(0) as i32;
                        score += 30 + (100 - pos.min(100)) / 10;
                    } else {
                        // Check other lines with decaying bonus.
                        let mut line_bonus = 8;
                        for line in e.text.lines().skip(1) {
                            if line.to_lowercase().contains(term) {
                                score += line_bonus;
                                break;
                            }
                            line_bonus = line_bonus.saturating_sub(4);
                        }
                    }
                    score -= (first_line.len() as i32) / 6;
                }
                Some((score, e))
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.path.cmp(&b.1.path)));

        let cat_set: BTreeSet<String> =
            scored.iter().map(|(_, e)| e.category.clone()).collect();
        let mut categories: Vec<String> = vec!["All".into()];
        categories.append(&mut cat_set.into_iter().collect());

        SearchResult {
            results: scored.into_iter().take(200).map(|(_, e)| e.clone()).collect(),
            categories,
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_vec_pretty(&self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let json = std::fs::read(path)?;
        Ok(serde_json::from_slice(&json)?)
    }
}

fn resolve_thumbnails(base: &Node, entries: &mut Vec<SearchEntry>) {
    for entry in entries.iter_mut() {
        let Some(thumb) = &entry.thumbnail_path.clone() else {
            continue;
        };
        // Verify the guessed thumbnail actually has image data.
        let valid = base.at_path(thumb).ok().map_or(false, |n| n.has_image_data());
        if valid {
            continue;
        }
        // Probe alternative paths manually (avoid n.path() which includes Base/ prefix).
        let alt: Option<String> = match entry.category.as_str() {
            "Mob" | "Npc" => ["stand/0", "stand/1", "move/0", "fly/0", "hit1/0", "hit/0", "die/0"]
                .iter()
                .find_map(|p| {
                    let path = format!("{}/{p}", entry.path);
                    base.at_path(&path).ok().filter(|n| n.has_image_data())?;
                    Some(format!("{0}/{p}", entry.path))
                }),
            "Reactor" => ["0/0", "1/0", "2/0", "3/0", "0", "1", "2", "3"]
                .iter()
                .find_map(|p| {
                    let path = format!("{0}/{p}", entry.path);
                    base.at_path(&path).ok().filter(|n| n.has_image_data())?;
                    Some(format!("{0}/{p}", entry.path))
                }),
            _ => None,
        };
        entry.thumbnail_path = alt;
    }
}

fn make_item_path(category: &str, id: &str) -> (String, Option<String>) {
    match category {
        "Consume" | "Install" | "Etc" | "Cash" => {
            // IDs may be 7 or 8 digits; data always uses 8 digits with leading zero
            let padded = if id.len() == 7 {
                format!("0{id}")
            } else {
                id.to_string()
            };
            let prefix = &padded[..4];
            let path = format!("Item/{category}/{prefix}.img/{padded}");
            let thumb = Some(format!("{path}/info/icon"));
            (path, thumb)
        }
        "Pet" => {
            let path = format!("Item/Pet/{id}.img");
            let thumb = Some(format!("{path}/info/icon"));
            (path, thumb)
        }
        "Mob" | "Npc" | "Reactor" => {
            // Data files use 7-character zero-padded IDs.
            let padded = format!("{id:0>7}");
            let path = format!("{category}/{padded}.img");
            let thumb = match category {
                "Reactor" => Some(format!("{path}/0/0")),
                _ => Some(format!("{path}/stand/0")),
            };
            (path, thumb)
        }
        _ => {
            (id.to_string(), None)
        }
    }
}

fn collect_text(node: &Node) -> String {
    let mut parts = Vec::new();
    if let Ok(n) = node.at_path("name") {
        if let Ok(s) = String::try_from(n) {
            if !s.is_empty() {
                parts.push(s);
            }
        }
    }
    if let Ok(n) = node.at_path("desc") {
        if let Ok(s) = String::try_from(n) {
            if !s.is_empty() {
                parts.push(s);
            }
        }
    }
    for (_, child) in node.children() {
        let cn = child.name();
        if cn != "name" && cn != "desc" {
            if let Ok(s) = String::try_from(child) {
                if !s.is_empty() {
                    parts.push(s);
                }
            }
        }
    }
    parts.join("\n")
}

fn build_string_index(
    base: &Node,
    entries: &mut Vec<SearchEntry>,
    indexed_ids: &mut HashSet<(String, String)>,
) -> usize {
    let mut count = 0;

    let sub_images: Vec<(&str, &str)> = vec![
        ("Mob", "Mob"),
        ("Npc", "Npc"),
        ("Consume", "Consume"),
        ("Ins", "Install"),
        ("Etc", "Etc"),
        ("Cash", "Cash"),
        ("Pet", "Pet"),
        ("Reactor", "Reactor"),
    ];

    for &(sub_img_name, category) in &sub_images {
        let path = format!("String/{sub_img_name}.img");
        let Ok(img_node) = base.at_path(&path) else {
            continue;
        };
        for (_, child) in img_node.children() {
            let id = child.name();
            let text = collect_text(&child);
            if text.is_empty() {
                continue;
            }
            let (data_path, thumbnail_path) = make_item_path(category, &id);
            entries.push(SearchEntry {
                path: data_path,
                name: id.clone(),
                text,
                category: category.to_string(),
                wz_file: format!("{category}.wz"),
                thumbnail_path,
            });
            indexed_ids.insert((category.to_string(), id));
            count += 1;
        }
    }

    if let Ok(eqp_img) = base.at_path("String/Eqp.img") {
        if let Ok(eqp_folder) = eqp_img.at_path("Eqp") {
            for (slot_name, slot_node) in eqp_folder.children() {
                let slot = slot_name.to_string();
                for (_, id_node) in slot_node.children() {
                    let id = id_node.name();
                    let text = collect_text(&id_node);
                    if text.is_empty() {
                        continue;
                    }
                    let padded_id = format!("{:0>8}", id);
                    let data_path = format!("Character/{slot}/{padded_id}.img");
                    entries.push(SearchEntry {
                        path: data_path.clone(),
                        name: id.clone(),
                        text,
                        category: "Equip".into(),
                        wz_file: "Character.wz".into(),
                        thumbnail_path: Some(format!("{data_path}/info/icon")),
                    });
                    indexed_ids.insert(("Equip".into(), id));
                    count += 1;
                }
            }
        }
    }

    if let Ok(quest_img) = base.at_path("String/Quest.img") {
        for (_, child) in quest_img.children() {
            let id = child.name();
            let text = collect_text(&child);
            if text.is_empty() {
                continue;
            }
            entries.push(SearchEntry {
                path: format!("Quest/quest/{id}"),
                name: id.clone(),
                text,
                category: "Quest".into(),
                wz_file: "Quest.wz".into(),
                thumbnail_path: None,
            });
            indexed_ids.insert(("Quest".into(), id));
            count += 1;
        }
    }

    if let Ok(book_img) = base.at_path("String/Book.img") {
        for (_, child) in book_img.children() {
            let id = child.name();
            let text = collect_text(&child);
            if text.is_empty() {
                continue;
            }
            entries.push(SearchEntry {
                path: format!("Book/{id}"),
                name: id.clone(),
                text,
                category: "Book".into(),
                wz_file: "String.wz".into(),
                thumbnail_path: None,
            });
            indexed_ids.insert(("Book".into(), id));
            count += 1;
        }
    }

    if let Ok(map_img) = base.at_path("String/Map.img") {
        for (_, child) in map_img.children() {
            let id = child.name();
            indexed_ids.insert(("Map".into(), id));
        }
    }

    count
}

fn build_map_name_mapping(base: &Node) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(map_img) = base.at_path("String/Map.img") else {
        return map;
    };
    for (_, region_node) in map_img.children() {
        for (_, id_node) in region_node.children() {
            let id = id_node.name();
            let mut parts = Vec::new();
            if let Ok(n) = id_node.at_path("streetName") {
                if let Ok(s) = String::try_from(n) {
                    parts.push(s);
                }
            }
            if let Ok(n) = id_node.at_path("mapName") {
                if let Ok(s) = String::try_from(n) {
                    parts.push(s);
                }
            }
            if !parts.is_empty() {
                map.insert(id, parts.join("\n"));
            }
        }
    }
    map
}

fn build_map_index(
    base: &Node,
    map_names: &HashMap<String, String>,
    entries: &mut Vec<SearchEntry>,
    indexed_ids: &mut HashSet<(String, String)>,
) -> usize {
    let mut count = 0;
    let Ok(map_top) = base.at_path("Map/Map") else {
        return 0;
    };
    for (_, level_dir) in map_top.children() {
        let level_name = level_dir.name();
        if !level_name.starts_with("Map") {
            continue;
        }
        for (_, child) in level_dir.children() {
            let name = child.name();
            if !name.ends_with(".img") {
                continue;
            }
            if child.try_parse().is_err() {
                continue;
            }
            let id = name.trim_end_matches(".img").to_string();

            let text = map_names
                .get(&id)
                .cloned()
                .unwrap_or_else(|| id.clone());

            let path = format!("Map/Map/{level_name}/{name}");
            entries.push(SearchEntry {
                path: path.clone(),
                name: id.clone(),
                text,
                category: "Map".into(),
                wz_file: "Map.wz".into(),
                thumbnail_path: Some(format!("{path}/miniMap/canvas")),
            });
            indexed_ids.insert(("Map".into(), id));
            count += 1;
        }
    }
    count
}

fn build_skill_mapping(base: &Node) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(skill_dir) = base.at_path("Skill") else {
        return map;
    };
    for (_, child) in skill_dir.children() {
        let class_name = child.name();
        if !class_name.ends_with(".img") {
            continue;
        }
        if child.try_parse().is_err() {
            continue;
        }
        let Ok(skill_folder) = child.at_path("skill") else {
            continue;
        };
        for (_, skill_node) in skill_folder.children() {
            let skill_id = skill_node.name();
            map.insert(skill_id.clone(), format!("Skill/{class_name}/skill/{skill_id}"));
        }
    }
    map
}

fn build_filename_index(
    base: &Node,
    entries: &mut Vec<SearchEntry>,
    indexed_ids: &HashSet<(String, String)>,
) -> usize {
    let mut count = 0;

    let dirs: Vec<(&str, &str, Option<&str>)> = vec![
        ("Mob", "Mob", Some("stand/0")),
        ("Npc", "Npc", Some("stand/0")),
        ("Reactor", "Reactor", Some("0/0")),
        ("Quest", "Quest", None),
        ("Effect", "Effect", None),
        ("UI", "UI", None),
        ("Sound", "Sound", None),
        ("TamingMob", "TamingMob", None),
        ("Morph", "Morph", None),
        ("Character", "Character", None),
    ];

    for &(dir_name, category, thumb_suffix) in &dirs {
        let Ok(dir_node) = base.at_path(dir_name) else {
            continue;
        };
        for (_, child) in dir_node.children() {
            let name = child.name();
            if !name.ends_with(".img") {
                continue;
            }
            let id = name.trim_end_matches(".img").to_string();
            if indexed_ids.contains(&(category.to_string(), id.clone())) {
                continue;
            }
            let text = id.clone();
            let path = format!("{dir_name}/{name}");
            let thumbnail_path = thumb_suffix.map(|s| format!("{path}/{s}"));
            entries.push(SearchEntry {
                path,
                name: id,
                text,
                category: category.to_string(),
                wz_file: format!("{category}.wz"),
                thumbnail_path,
            });
            count += 1;
        }
    }

    let item_dirs: Vec<(&str, &str, Option<&str>)> = vec![
        ("Consume", "Consume", Some("info/icon")),
        ("Install", "Install", Some("info/icon")),
        ("Etc", "Etc", Some("info/icon")),
        ("Cash", "Cash", Some("info/icon")),
        ("Pet", "Pet", Some("info/icon")),
        ("Special", "Special", Some("info/icon")),
    ];

    let Ok(item_dir) = base.at_path("Item") else {
        return count;
    };
    for &(sub_dir, category, thumb_suffix) in &item_dirs {
        let Ok(sub_node) = item_dir.at_path(sub_dir) else {
            continue;
        };
        for (_, child) in sub_node.children() {
            let name = child.name();
            if !name.ends_with(".img") {
                continue;
            }
            let id = name.trim_end_matches(".img").to_string();
            if indexed_ids.contains(&(category.to_string(), id.clone())) {
                continue;
            }
            let text = id.clone();
            let path = format!("Item/{sub_dir}/{name}");
            let thumbnail_path = thumb_suffix.map(|s| format!("{path}/{s}"));
            entries.push(SearchEntry {
                path,
                name: id,
                text,
                category: category.to_string(),
                wz_file: "Item.wz".into(),
                thumbnail_path,
            });
            count += 1;
        }
    }
    count
}
