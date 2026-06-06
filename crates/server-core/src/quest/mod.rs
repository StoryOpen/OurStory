pub mod dialog;
pub mod eval;
pub mod loader;
pub mod types;

use crate::player::{ActiveQuest, Player};
use protocol::types::{DialogBranch, ObjectiveInfo, ObjectiveType, QuestDialog, StopReason};
use std::collections::HashMap;
use std::time::Instant;
use types::QuestDef;

pub struct QuestRegistry {
    quests: HashMap<u32, QuestDef>,
}

impl QuestRegistry {
    pub fn load(base: &wz_reader::node::WzNodeArc) -> Self {
        let quests = loader::load_quest_registry(base);
        tracing::info!("Loaded {} quest definitions", quests.len());
        Self { quests }
    }

    pub fn get(&self, id: u32) -> Option<&QuestDef> {
        self.quests.get(&id)
    }

    pub fn available_starts(&self, player: &Player, npc_id: u32) -> Vec<u32> {
        self.quests
            .values()
            .filter(|q| {
                if player.active_quests.iter().any(|aq| aq.quest_id == q.id) {
                    return false;
                }
                if player.completed_quests.contains(&q.id) {
                    return false;
                }
                if q.start_check.npc_id != Some(npc_id) {
                    return false;
                }
                if q.start_script.is_some() || q.start_check.has_script {
                    return true;
                }
                matches!(eval::can_start(player, q), eval::CheckResult::Pass)
            })
            .map(|q| q.id)
            .collect()
    }

    pub fn available_completions(&self, player: &Player, npc_id: u32) -> Vec<u32> {
        self.quests
            .values()
            .filter(|q| {
                let active = player.active_quests.iter().find(|aq| aq.quest_id == q.id);
                let aq = match active {
                    Some(aq) => aq,
                    None => return false,
                };
                if q.complete_check.npc_id != Some(npc_id) {
                    return false;
                }
                if q.end_script.is_some() || q.complete_check.has_script {
                    return true;
                }
                matches!(
                    eval::check_completion_conditions(&q.complete_check, player, Some(aq)),
                    eval::CheckResult::Pass
                )
            })
            .map(|q| q.id)
            .collect()
    }

    pub fn start_quest(
        &self,
        player: &mut Player,
        quest_id: u32,
    ) -> Result<QuestStartedResult, QuestError> {
        let quest = self.quests.get(&quest_id).ok_or(QuestError::NotFound)?;

        if player
            .active_quests
            .iter()
            .any(|aq| aq.quest_id == quest_id)
        {
            return Err(QuestError::AlreadyActive);
        }
        if player.completed_quests.contains(&quest_id) {
            return Err(QuestError::AlreadyCompleted);
        }

        if quest.start_script.is_some() || quest.start_check.has_script {
            return Err(QuestError::ScriptedQuest);
        }

        match eval::can_start(player, quest) {
            eval::CheckResult::Pass => {}
            eval::CheckResult::Fail => {
                let reason = eval::get_start_failed_reason(&quest.start_check, player);
                return Err(QuestError::ConditionsNotMet(
                    reason.unwrap_or("unknown").to_string(),
                ));
            }
            eval::CheckResult::HasScript => return Err(QuestError::ScriptedQuest),
        }

        let mut items_given = Vec::new();
        for item in &quest.start_act.items {
            if item.count > 0 {
                items_given.push(protocol::types::ItemGrant {
                    item_id: item.item_id,
                    count: item.count,
                    period_minutes: item.period_minutes,
                });
            }
        }

        let objectives = build_objectives(quest);

        player.active_quests.push(ActiveQuest {
            quest_id,
            kill_counts: HashMap::new(),
            started_at: Instant::now(),
        });

        let dialog = dialog::start_dialog(quest_id, true, None);

        Ok(QuestStartedResult {
            quest_id,
            objectives,
            dialog,
            items_given,
        })
    }

    pub fn complete_quest(
        &self,
        player: &mut Player,
        quest_id: u32,
    ) -> Result<QuestCompletedResult, QuestError> {
        let quest = self.quests.get(&quest_id).ok_or(QuestError::NotFound)?;

        let active_idx = player
            .active_quests
            .iter()
            .position(|aq| aq.quest_id == quest_id)
            .ok_or(QuestError::NotActive)?;

        if quest.end_script.is_some() || quest.complete_check.has_script {
            return Err(QuestError::ScriptedQuest);
        }

        let aq = &player.active_quests[active_idx];
        match eval::check_completion_conditions(&quest.complete_check, player, Some(aq)) {
            eval::CheckResult::Pass => {}
            eval::CheckResult::Fail => {
                let reason =
                    eval::get_complete_failed_reason(&quest.complete_check, player, Some(aq));
                return Err(QuestError::ConditionsNotMet(
                    reason.unwrap_or("unknown").to_string(),
                ));
            }
            eval::CheckResult::HasScript => return Err(QuestError::ScriptedQuest),
        }

        let mut items_given = Vec::new();
        for item in &quest.complete_act.items {
            if item.count > 0 {
                items_given.push(protocol::types::ItemGrant {
                    item_id: item.item_id,
                    count: item.count,
                    period_minutes: item.period_minutes,
                });
            }
        }

        let next_quest = quest.complete_act.next_quest;

        player.active_quests.remove(active_idx);
        player.completed_quests.insert(quest_id);

        let dialog = dialog::complete_dialog(quest_id, true, None);

        Ok(QuestCompletedResult {
            quest_id,
            exp: quest.complete_act.exp,
            items_given,
            next_quest,
            dialog,
        })
    }

    pub fn forfeit_quest(&self, player: &mut Player, quest_id: u32) -> Result<(), QuestError> {
        let quest = self.quests.get(&quest_id).ok_or(QuestError::NotFound)?;

        let active_idx = player
            .active_quests
            .iter()
            .position(|aq| aq.quest_id == quest_id)
            .ok_or(QuestError::NotActive)?;

        let mut items_to_reclaim = Vec::new();
        for item in &quest.start_act.items {
            if item.count > 0 {
                items_to_reclaim.push(protocol::types::ItemGrant {
                    item_id: item.item_id,
                    count: -item.count,
                    period_minutes: item.period_minutes,
                });
            }
        }

        player.active_quests.remove(active_idx);

        Ok(())
    }

    pub fn on_mob_killed(&self, player: &mut Player, mob_id: u32) -> Vec<QuestProgressUpdate> {
        let mut updates = Vec::new();

        let quest_ids: Vec<u32> = player.active_quests.iter().map(|aq| aq.quest_id).collect();

        for quest_id in quest_ids {
            if let Some(quest) = self.quests.get(&quest_id) {
                let mut killed_any = false;
                let mut required_count = 0u32;

                for (req_mob, required) in &quest.complete_check.required_kills {
                    if *req_mob == mob_id {
                        if let Some(aq) = player
                            .active_quests
                            .iter_mut()
                            .find(|aq| aq.quest_id == quest_id)
                        {
                            let count = aq.kill_counts.entry(mob_id).or_insert(0);
                            *count += 1;
                            killed_any = true;
                            required_count = *required;
                        }
                    }
                }

                if killed_any {
                    let aq = player
                        .active_quests
                        .iter()
                        .find(|aq| aq.quest_id == quest_id);
                    let current = aq
                        .and_then(|aq| aq.kill_counts.get(&mob_id).copied())
                        .unwrap_or(0);
                    let completable = aq.is_some_and(|aq| {
                        eval::check_completion_conditions(&quest.complete_check, player, Some(aq))
                            == eval::CheckResult::Pass
                    });

                    updates.push(QuestProgressUpdate {
                        quest_id,
                        mob_id,
                        current,
                        target: required_count,
                        completable,
                    });
                }
            }
        }

        updates
    }

    pub fn get_objectives(&self, player: &Player, quest_id: u32) -> Vec<ObjectiveInfo> {
        let aq = player
            .active_quests
            .iter()
            .find(|aq| aq.quest_id == quest_id);
        match aq {
            Some(aq) => {
                if let Some(quest) = self.quests.get(&quest_id) {
                    let mut objectives = Vec::new();
                    for (mob_id, required) in &quest.complete_check.required_kills {
                        let current = aq.kill_counts.get(mob_id).copied().unwrap_or(0);
                        objectives.push(ObjectiveInfo {
                            obj_type: ObjectiveType::Kill,
                            target_id: *mob_id,
                            current,
                            target: *required,
                        });
                    }
                    for (item_id, count) in &quest.complete_check.required_items {
                        objectives.push(ObjectiveInfo {
                            obj_type: ObjectiveType::Item,
                            target_id: *item_id,
                            current: 0,
                            target: *count,
                        });
                    }
                    objectives
                } else {
                    vec![]
                }
            }
            None => vec![],
        }
    }

    pub fn get_dialog(
        &self,
        player: &Player,
        quest_id: u32,
        is_start: bool,
        accept: bool,
    ) -> QuestDialog {
        let quest = match self.quests.get(&quest_id) {
            Some(q) => q,
            None => {
                return QuestDialog {
                    quest_id,
                    stage: if is_start { 0 } else { 1 },
                    branch: DialogBranch::Stop {
                        reason: StopReason::Generic,
                    },
                };
            }
        };

        if is_start {
            if accept {
                dialog::start_dialog(quest_id, true, None)
            } else {
                dialog::start_dialog(quest_id, false, None)
            }
        } else {
            let aq = player
                .active_quests
                .iter()
                .find(|aq| aq.quest_id == quest_id);
            let completable = match aq {
                Some(aq) => {
                    eval::check_completion_conditions(&quest.complete_check, player, Some(aq))
                        == eval::CheckResult::Pass
                }
                None => false,
            };
            let reason = eval::get_complete_failed_reason(&quest.complete_check, player, aq);
            dialog::complete_dialog(quest_id, completable, reason)
        }
    }
}

pub struct QuestStartedResult {
    pub quest_id: u32,
    pub objectives: Vec<ObjectiveInfo>,
    pub dialog: QuestDialog,
    pub items_given: Vec<protocol::types::ItemGrant>,
}

pub struct QuestCompletedResult {
    pub quest_id: u32,
    pub exp: u32,
    pub items_given: Vec<protocol::types::ItemGrant>,
    pub next_quest: Option<u32>,
    pub dialog: QuestDialog,
}

pub struct QuestProgressUpdate {
    pub quest_id: u32,
    pub mob_id: u32,
    pub current: u32,
    pub target: u32,
    pub completable: bool,
}

#[derive(Debug)]
pub enum QuestError {
    NotFound,
    AlreadyActive,
    AlreadyCompleted,
    NotActive,
    ConditionsNotMet(String),
    ScriptedQuest,
}

fn build_objectives(quest: &QuestDef) -> Vec<ObjectiveInfo> {
    let mut objectives = Vec::new();
    for (mob_id, required) in &quest.complete_check.required_kills {
        objectives.push(ObjectiveInfo {
            obj_type: ObjectiveType::Kill,
            target_id: *mob_id,
            current: 0,
            target: *required,
        });
    }
    for (item_id, count) in &quest.complete_check.required_items {
        objectives.push(ObjectiveInfo {
            obj_type: ObjectiveType::Item,
            target_id: *item_id,
            current: 0,
            target: *count,
        });
    }
    objectives
}
