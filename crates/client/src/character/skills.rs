use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum SkillType {
    Passive, Active, AttackProc, Special,
}

#[derive(Debug, Clone, Reflect)]
pub struct EffectFrame {
    pub image: Handle<Image>,
    pub origin: Vec2,
    pub z: i32,
    pub delay: u32,
    pub alpha0: Option<u8>,
    pub alpha1: Option<u8>,
}

#[derive(Debug, Clone, Reflect)]
pub struct SkillLevelData {
    pub damage: Option<i32>,
    pub mp_con: Option<i32>,
    pub hp_con: Option<i32>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub time: Option<i32>,
    pub prop: Option<i32>,
    pub pad: Option<i32>,
    pub mad: Option<i32>,
    pub pdd: Option<i32>,
    pub mdd: Option<i32>,
    pub acc: Option<i32>,
    pub eva: Option<i32>,
    pub speed: Option<i32>,
    pub jump: Option<i32>,
    pub hs: Option<String>,
}

#[derive(Debug, Clone, Reflect)]
pub struct SkillEntry {
    pub id: u32,
    pub skill_type: SkillType,
    pub name: String,
    pub desc: String,
    pub levels: HashMap<u32, SkillLevelData>,
    pub action: Option<String>,
    pub prepare_action: Option<String>,
    pub effect_frames: Vec<EffectFrame>,
    pub hit_frames: Vec<EffectFrame>,
    pub keydown_frames: Vec<EffectFrame>,
    pub icon: Handle<Image>,
    pub icon_disabled: Handle<Image>,
    pub icon_mouse_over: Handle<Image>,
    pub req: HashMap<u32, u32>,
    pub master_level: u32,
    pub invisible: bool,
    pub skill_type_raw: Option<i32>,
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct SkillDatabase {
    pub skills: HashMap<u32, SkillEntry>,
}

impl SkillDatabase {
    pub fn load(wz: &wz::WzData) -> Self {
        let db = match wz.load_skill_database() {
            Ok(db) => Some(db),
            Err(e) => {
                warn!("SkillDatabase::load: failed to load skill database: {e}, using empty");
                None
            }
        };
        let mut skills = HashMap::new();

        if let Some(db) = db {
            for (id, entry) in &db.skills {
                skills.insert(*id, SkillEntry {
                    id: entry.id,
                    skill_type: match entry.skill_type {
                        wz::SkillType::Passive => SkillType::Passive,
                        wz::SkillType::Active => SkillType::Active,
                        wz::SkillType::AttackProc => SkillType::AttackProc,
                        wz::SkillType::Special => SkillType::Special,
                    },
                    name: entry.name.clone(),
                    desc: entry.desc.clone(),
                    levels: entry.levels.iter().map(|(k, v)| (*k, SkillLevelData {
                        damage: v.damage, mp_con: v.mp_con, hp_con: v.hp_con,
                        x: v.x, y: v.y, time: v.time, prop: v.prop,
                        pad: v.pad, mad: v.mad, pdd: v.pdd, mdd: v.mdd,
                        acc: v.acc, eva: v.eva, speed: v.speed, jump: v.jump,
                        hs: v.hs.clone(),
                    })).collect(),
                    action: entry.action.clone(),
                    prepare_action: entry.prepare_action.clone(),
                    effect_frames: Vec::new(),
                    hit_frames: Vec::new(),
                    keydown_frames: Vec::new(),
                    icon: Handle::default(),
                    icon_disabled: Handle::default(),
                    icon_mouse_over: Handle::default(),
                    req: entry.req.clone(),
                    master_level: entry.master_level,
                    invisible: entry.invisible,
                    skill_type_raw: entry.skill_type_raw,
                });
            }
        }

        SkillDatabase { skills }
    }

    pub fn get(&self, id: u32) -> Option<&SkillEntry> {
        self.skills.get(&id)
    }

    pub fn skills_for_job(&self, job_lineage: &[crate::character::job::Job]) -> Vec<&SkillEntry> {
        let mut result = Vec::new();
        for job in job_lineage {
            for skill in self.skills.values() {
                if skill.id / 10000 == job.0 {
                    result.push(skill);
                }
            }
        }
        result
    }

    pub fn active_skills_for_job(&self, job_lineage: &[crate::character::job::Job]) -> Vec<&SkillEntry> {
        self.skills_for_job(job_lineage)
            .into_iter()
            .filter(|s| matches!(s.skill_type, SkillType::Active | SkillType::Special))
            .collect()
    }
}

#[derive(Component, Default, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct LearnedSkills {
    pub skills: HashMap<u32, u32>,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct SkillEffect {
    pub frames: Vec<EffectFrame>,
    pub frame_idx: usize,
    pub timer: Timer,
    pub finished: bool,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct SkillEffectRoot;
