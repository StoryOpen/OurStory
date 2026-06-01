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
