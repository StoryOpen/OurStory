use super::types::*;
use std::collections::HashMap;
use wz_reader::node::WzNodeArc;
use wz_reader::WzNodeCast;

pub fn load_quest_registry(base: &WzNodeArc) -> HashMap<u32, QuestDef> {
    let mut quests = HashMap::new();

    let info_node = match wz::resolve_path(base, "Quest/QuestInfo.img") {
        Some(n) => n,
        None => return quests,
    };
    let check_node = wz::resolve_path(base, "Quest/Check.img");
    let act_node = wz::resolve_path(base, "Quest/Act.img");

    for (id_str, info_child) in wz::get_children(&info_node) {
        let Ok(quest_id) = id_str.parse::<u32>() else {
            continue;
        };

        let name = get_string_child(&info_child, "name").unwrap_or_default();
        let area = get_int_child(&info_child, "area").unwrap_or(0) as u32;
        let auto_start = get_int_child(&info_child, "autoStart").unwrap_or(0) != 0;
        let auto_complete = get_int_child(&info_child, "autoComplete").unwrap_or(0) != 0;

        let start_check = check_node
            .as_ref()
            .and_then(|n| wz::resolve_path(n, &format!("{quest_id}/0")))
            .map(|n| parse_check_conditions(&n))
            .unwrap_or_default();

        let complete_check = check_node
            .as_ref()
            .and_then(|n| wz::resolve_path(n, &format!("{quest_id}/1")))
            .map(|n| parse_check_conditions(&n))
            .unwrap_or_default();

        let start_act = act_node
            .as_ref()
            .and_then(|n| wz::resolve_path(n, &format!("{quest_id}/0")))
            .map(|n| parse_quest_actions(&n))
            .unwrap_or_default();

        let complete_act = act_node
            .as_ref()
            .and_then(|n| wz::resolve_path(n, &format!("{quest_id}/1")))
            .map(|n| parse_quest_actions(&n))
            .unwrap_or_default();

        let start_script = check_node
            .as_ref()
            .and_then(|n| wz::resolve_path(n, &format!("{quest_id}/0/startscript")))
            .and_then(|n| n.read().ok()?.try_as_string()?.get_string().ok())
            .map(|s| s.to_string());

        let end_script = check_node
            .as_ref()
            .and_then(|n| wz::resolve_path(n, &format!("{quest_id}/1/endscript")))
            .and_then(|n| n.read().ok()?.try_as_string()?.get_string().ok())
            .map(|s| s.to_string());

        quests.insert(
            quest_id,
            QuestDef {
                id: quest_id,
                name,
                area,
                auto_start,
                auto_complete,
                start_check,
                complete_check,
                start_act,
                complete_act,
                start_script,
                end_script,
            },
        );
    }

    quests
}

fn parse_check_conditions(node: &WzNodeArc) -> CheckConditions {
    let mut conds = CheckConditions::default();

    conds.npc_id = get_int_child(node, "npc").map(|v| v as u32);
    conds.level_min = get_int_child(node, "lvmin").map(|v| v as u32);
    conds.level_max = get_int_child(node, "lvmax").map(|v| v as u32);
    conds.normal_auto_start = get_int_child(node, "normalAutoStart").unwrap_or(0) != 0;
    conds.cooldown_minutes = get_int_child(node, "interval").map(|v| v as u32);
    conds.time_start = get_string_child(node, "start");
    conds.time_end = get_string_child(node, "end");

    if let Some(job_node) = wz::resolve_path(node, "job") {
        let jobs: Vec<u32> = wz::get_children(&job_node)
            .iter()
            .filter_map(|(_, child)| child.read().ok()?.try_as_int().map(|v| *v as u32))
            .collect();
        if !jobs.is_empty() {
            conds.job_whitelist = Some(jobs);
        }
    }

    if let Some(quest_node) = wz::resolve_path(node, "quest") {
        for (_, child) in wz::get_children(&quest_node) {
            let id = get_int_child(&child, "id").unwrap_or(0) as u32;
            let state = get_int_child(&child, "state").unwrap_or(2) as u32;
            conds.prerequisite_quests.push((id, state));
        }
    }

    if let Some(item_node) = wz::resolve_path(node, "item") {
        for (_, child) in wz::get_children(&item_node) {
            let id = get_int_child(&child, "id").unwrap_or(0) as u32;
            let count = get_int_child(&child, "count").unwrap_or(1);
            conds.required_items.push((id, count as u32));
        }
    }

    if let Some(mob_node) = wz::resolve_path(node, "mob") {
        for (_, child) in wz::get_children(&mob_node) {
            let id = get_int_child(&child, "id").unwrap_or(0) as u32;
            let count = get_int_child(&child, "count").unwrap_or(1) as u32;
            conds.required_kills.push((id, count));
        }
    }

    if let Some(skill_node) = wz::resolve_path(node, "skill") {
        for (_, child) in wz::get_children(&skill_node) {
            let id = get_int_child(&child, "id").unwrap_or(0) as u32;
            let acquire = get_int_child(&child, "acquire").unwrap_or(0) != 0;
            conds.required_skills.push((id, acquire));
        }
    }

    conds.has_script = wz::resolve_path(node, "startscript").is_some()
        || wz::resolve_path(node, "endscript").is_some();

    conds
}

fn parse_quest_actions(node: &WzNodeArc) -> QuestActions {
    let mut actions = QuestActions::default();

    actions.exp = get_int_child(node, "exp").unwrap_or(0) as u32;
    actions.next_quest = get_int_child(node, "nextQuest").map(|v| v as u32);
    actions.npc_act = get_string_child(node, "npcAct");
    actions.pet_speed = get_int_child(node, "petspeed").map(|v| v as u32);

    if let Some(item_node) = wz::resolve_path(node, "item") {
        for (_, child) in wz::get_children(&item_node) {
            let item_id = get_int_child(&child, "id").unwrap_or(0) as u32;
            let count = get_int_child(&child, "count").unwrap_or(0);
            let period = get_int_child(&child, "period").map(|v| v as u32);
            let job_filter = get_int_child(&child, "job").map(|v| v as u32);
            actions.items.push(ItemAction {
                item_id,
                count,
                period_minutes: period,
                job_filter,
            });
        }
    }

    if let Some(skill_node) = wz::resolve_path(node, "skill") {
        for (_, child) in wz::get_children(&skill_node) {
            let skill_id = get_int_child(&child, "id").unwrap_or(0) as u32;
            let skill_level = get_int_child(&child, "skillLevel").unwrap_or(1) as u32;
            let master_level = get_int_child(&child, "masterLevel").map(|v| v as u32);

            let job_whitelist = if let Some(job_node) = wz::resolve_path(&child, "job") {
                wz::get_children(&job_node)
                    .iter()
                    .filter_map(|(_, c)| c.read().ok()?.try_as_int().map(|v| *v as u32))
                    .collect()
            } else {
                vec![]
            };

            actions.skill_grants.push(SkillGrant {
                skill_id,
                skill_level,
                master_level,
                job_whitelist,
            });
        }
    }

    actions
}

fn get_int_child(node: &WzNodeArc, name: &str) -> Option<i32> {
    let child = wz::resolve_path(node, name)?;
    child.read().ok()?.try_as_int().copied()
}

fn get_string_child(node: &WzNodeArc, name: &str) -> Option<String> {
    let child = wz::resolve_path(node, name)?;
    child
        .read()
        .ok()?
        .try_as_string()?
        .get_string()
        .ok()
        .map(|s| s.to_string())
}
