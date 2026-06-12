use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkillType {
    Passive,
    Active,
    AttackProc,
    Special,
}

impl SkillType {
    pub fn from_raw(raw: i32) -> Self {
        match raw {
            1 => SkillType::Passive,
            2 => SkillType::Active,
            3 => SkillType::AttackProc,
            4 => SkillType::Special,
            _ => SkillType::Active,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EffectFrame {
    pub image: Handle<Image>,
    pub origin: Vec2,
    pub z: i32,
    pub delay: u32,
    pub alpha0: Option<u8>,
    pub alpha1: Option<u8>,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

#[derive(Resource)]
pub struct SkillDatabase {
    pub skills: HashMap<u32, SkillEntry>,
}

impl SkillDatabase {
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

#[derive(Component, Default, Debug, Clone)]
pub struct LearnedSkills {
    pub skills: HashMap<u32, u32>,
}

#[derive(Component)]
pub struct SkillEffect {
    pub frames: Vec<EffectFrame>,
    pub frame_idx: usize,
    pub timer: Timer,
    pub finished: bool,
}

#[derive(Component)]
pub struct SkillEffectRoot;
