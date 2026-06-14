use log::warn;
use std::collections::HashMap;
use crate::error::WzError;
use crate::node::Node;

#[derive(Debug, Clone)]
pub struct QuestRegistry {
    pub quests: HashMap<u32, QuestDef>,
}

#[derive(Debug, Clone)]
pub struct QuestDef {
    pub id: u32,
    pub name: String,
    pub area: u32,
    pub auto_start: bool,
    pub auto_complete: bool,
    pub start_check: CheckConditions,
    pub complete_check: CheckConditions,
    pub start_act: QuestActions,
    pub complete_act: QuestActions,
    pub start_script: Option<String>,
    pub end_script: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct CheckConditions {
    pub npc_id: Option<u32>,
    pub level_min: Option<u32>,
    pub level_max: Option<u32>,
    pub job_whitelist: Option<Vec<u32>>,
    pub prerequisite_quests: Vec<(u32, u32)>,
    pub required_items: Vec<(u32, u32)>,
    pub required_kills: Vec<(u32, u32)>,
    pub required_skills: Vec<(u32, bool)>,
    pub cooldown_minutes: Option<u32>,
    pub time_start: Option<String>,
    pub time_end: Option<String>,
    pub normal_auto_start: bool,
    pub has_script: bool,
}

#[derive(Debug, Clone, Default)]
pub struct QuestActions {
    pub exp: u32,
    pub items: Vec<ItemAction>,
    pub next_quest: Option<u32>,
    pub npc_act: Option<String>,
    pub skill_grants: Vec<SkillGrant>,
    pub pet_speed: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ItemAction {
    pub item_id: u32,
    pub count: i32,
    pub period_minutes: Option<u32>,
    pub job_filter: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct SkillGrant {
    pub skill_id: u32,
    pub skill_level: u32,
    pub master_level: Option<u32>,
    pub job_whitelist: Vec<u32>,
}

impl QuestRegistry {
    pub(crate) fn load(base: &Node) -> Result<Self, WzError> {
        let mut quests = HashMap::new();

        let info_node = match base.at_path("Quest/QuestInfo.img") {
            Ok(n) => n,
            Err(_) => return Ok(QuestRegistry { quests }),
        };
        let check_node = base.at_path("Quest/Check.img").ok();
        let act_node = base.at_path("Quest/Act.img").ok();

        for (id_str, info_child) in info_node.children() {
            let Ok(quest_id) = id_str.to_string().parse::<u32>() else { continue; };

            let name = info_child.get_opt::<String>("name").unwrap_or_else(|| {
                warn!("Quest {quest_id}: name missing, using default");
                String::new()
            });
            let area = info_child.get_opt::<i32>("area").unwrap_or_else(|| {
                warn!("Quest {quest_id}: area missing, using 0");
                0
            }) as u32;
            let auto_start = info_child.get_opt::<i32>("autoStart").unwrap_or_else(|| {
                warn!("Quest {quest_id}: autoStart missing, using 0");
                0
            }) != 0;
            let auto_complete = info_child.get_opt::<i32>("autoComplete").unwrap_or_else(|| {
                warn!("Quest {quest_id}: autoComplete missing, using 0");
                0
            }) != 0;

            let start_check = check_node.as_ref()
                .and_then(|n| n.at_path(&format!("{quest_id}/0")).ok())
                .map(|n| parse_check_conditions(&n))
                .unwrap_or_else(|| {
                    warn!("Quest {quest_id}: start_check missing, using default");
                    CheckConditions::default()
                });

            let complete_check = check_node.as_ref()
                .and_then(|n| n.at_path(&format!("{quest_id}/1")).ok())
                .map(|n| parse_check_conditions(&n))
                .unwrap_or_else(|| {
                    warn!("Quest {quest_id}: complete_check missing, using default");
                    CheckConditions::default()
                });

            let start_act = act_node.as_ref()
                .and_then(|n| n.at_path(&format!("{quest_id}/0")).ok())
                .map(|n| parse_quest_actions(&n))
                .unwrap_or_else(|| {
                    warn!("Quest {quest_id}: start_act missing, using default");
                    QuestActions::default()
                });

            let complete_act = act_node.as_ref()
                .and_then(|n| n.at_path(&format!("{quest_id}/1")).ok())
                .map(|n| parse_quest_actions(&n))
                .unwrap_or_else(|| {
                    warn!("Quest {quest_id}: complete_act missing, using default");
                    QuestActions::default()
                });

            let start_script = check_node.as_ref()
                .and_then(|n| n.at_path(&format!("{quest_id}/0/startscript")).ok())
                .and_then(|n| n.try_into().ok());

            let end_script = check_node.as_ref()
                .and_then(|n| n.at_path(&format!("{quest_id}/1/endscript")).ok())
                .and_then(|n| n.try_into().ok());

            quests.insert(quest_id, QuestDef {
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
            });
        }

        Ok(QuestRegistry { quests })
    }
}

fn parse_check_conditions(node: &Node) -> CheckConditions {
    let mut conds = CheckConditions::default();

    conds.npc_id = node.get_opt::<i32>("npc").map(|v| v as u32);
    conds.level_min = node.get_opt::<i32>("lvmin").map(|v| v as u32);
    conds.level_max = node.get_opt::<i32>("lvmax").map(|v| v as u32);
    conds.normal_auto_start = node.get_opt::<i32>("normalAutoStart").unwrap_or_else(|| {
        warn!("parse_check_conditions: normalAutoStart missing, using 0");
        0
    }) != 0;
    conds.cooldown_minutes = node.get_opt::<i32>("interval").map(|v| v as u32);
    conds.time_start = node.get_opt("start");
    conds.time_end = node.get_opt("end");

    if let Ok(job_node) = node.at_path("job") {
        let jobs: Vec<u32> = job_node.children().into_iter()
            .filter_map(|(_, child)| {
                let v: Result<i32, _> = child.try_into();
                v.ok().map(|v| v as u32)
            })
            .collect();
        if !jobs.is_empty() {
            conds.job_whitelist = Some(jobs);
        }
    }

    if let Ok(quest_node) = node.at_path("quest") {
        for (_, child) in quest_node.children() {
            let id = child.get_opt::<i32>("id").unwrap_or_else(|| {
                warn!("parse_check_conditions: prereq quest id missing, using 0");
                0
            }) as u32;
            let state = child.get_opt::<i32>("state").unwrap_or_else(|| {
                warn!("parse_check_conditions: prereq quest state missing, using 2");
                2
            }) as u32;
            conds.prerequisite_quests.push((id, state));
        }
    }

    if let Ok(item_node) = node.at_path("item") {
        for (_, child) in item_node.children() {
            let id = child.get_opt::<i32>("id").unwrap_or_else(|| {
                warn!("parse_check_conditions: req item id missing, using 0");
                0
            }) as u32;
            let count = child.get_opt::<i32>("count").unwrap_or_else(|| {
                warn!("parse_check_conditions: req item count missing, using 1");
                1
            });
            conds.required_items.push((id, count as u32));
        }
    }

    if let Ok(mob_node) = node.at_path("mob") {
        for (_, child) in mob_node.children() {
            let id = child.get_opt::<i32>("id").unwrap_or_else(|| {
                warn!("parse_check_conditions: req mob id missing, using 0");
                0
            }) as u32;
            let count = child.get_opt::<i32>("count").unwrap_or_else(|| {
                warn!("parse_check_conditions: req mob count missing, using 1");
                1
            }) as u32;
            conds.required_kills.push((id, count));
        }
    }

    if let Ok(skill_node) = node.at_path("skill") {
        for (_, child) in skill_node.children() {
            let id = child.get_opt::<i32>("id").unwrap_or_else(|| {
                warn!("parse_check_conditions: req skill id missing, using 0");
                0
            }) as u32;
            let acquire = child.get_opt::<i32>("acquire").unwrap_or_else(|| {
                warn!("parse_check_conditions: req skill acquire missing, using 0");
                0
            }) != 0;
            conds.required_skills.push((id, acquire));
        }
    }

    conds.has_script = node.at_path("startscript").is_ok() || node.at_path("endscript").is_ok();

    conds
}

fn parse_quest_actions(node: &Node) -> QuestActions {
    let mut actions = QuestActions::default();

    actions.exp = node.get_opt::<i32>("exp").unwrap_or_else(|| {
        warn!("parse_quest_actions: exp missing, using 0");
        0
    }) as u32;
    actions.next_quest = node.get_opt::<i32>("nextQuest").map(|v| v as u32);
    actions.npc_act = node.get_opt("npcAct");
    actions.pet_speed = node.get_opt::<i32>("petspeed").map(|v| v as u32);

    if let Ok(item_node) = node.at_path("item") {
        for (_, child) in item_node.children() {
            let item_id = child.get_opt::<i32>("id").unwrap_or_else(|| {
                warn!("parse_quest_actions: action item id missing, using 0");
                0
            }) as u32;
            let count = child.get_opt::<i32>("count").unwrap_or_else(|| {
                warn!("parse_quest_actions: action item count missing, using 0");
                0
            });
            let period = child.get_opt::<i32>("period").map(|v| v as u32);
            let job_filter = child.get_opt::<i32>("job").map(|v| v as u32);
            actions.items.push(ItemAction { item_id, count, period_minutes: period, job_filter });
        }
    }

    if let Ok(skill_node) = node.at_path("skill") {
        for (_, child) in skill_node.children() {
            let skill_id = child.get_opt::<i32>("id").unwrap_or_else(|| {
                warn!("parse_quest_actions: action skill id missing, using 0");
                0
            }) as u32;
            let skill_level = child.get_opt::<i32>("skillLevel").unwrap_or_else(|| {
                warn!("parse_quest_actions: action skill level missing, using 1");
                1
            }) as u32;
            let master_level = child.get_opt::<i32>("masterLevel").map(|v| v as u32);
            let job_whitelist = child.at_path("job").ok()
                .map(|jn| jn.children().into_iter()
                    .filter_map(|(_, c)| {
                        let v: Result<i32, _> = c.try_into();
                        v.ok().map(|v| v as u32)
                    })
                    .collect())
                .unwrap_or_else(|| {
                    warn!("parse_quest_actions: action skill job whitelist missing, using default");
                    Vec::new()
                });
            actions.skill_grants.push(SkillGrant { skill_id, skill_level, master_level, job_whitelist });
        }
    }

    actions
}
