use super::types::*;
use crate::player::Player;
use protocol::types::Job;

#[derive(PartialEq)]
pub enum CheckResult {
    Pass,
    Fail,
    HasScript,
}

pub fn can_start(player: &Player, quest: &QuestDef) -> CheckResult {
    if quest.start_script.is_some() || quest.start_check.has_script {
        return CheckResult::HasScript;
    }
    evaluate(&quest.start_check, player)
}

pub fn can_complete(player: &Player, quest: &QuestDef) -> CheckResult {
    if quest.end_script.is_some() || quest.complete_check.has_script {
        return CheckResult::HasScript;
    }
    evaluate(&quest.complete_check, player)
}

fn evaluate(conds: &CheckConditions, player: &Player) -> CheckResult {
    if let Some(npc) = conds.npc_id {
        let _ = npc;
    }

    if let Some(lvmin) = conds.level_min {
        if (player.level as u32) < lvmin {
            return CheckResult::Fail;
        }
    }

    if let Some(lvmax) = conds.level_max {
        if (player.level as u32) > lvmax {
            return CheckResult::Fail;
        }
    }

    if let Some(ref jobs) = conds.job_whitelist {
        let player_job_id = job_to_id(player.job);
        if !jobs.contains(&player_job_id) {
            return CheckResult::Fail;
        }
    }

    for (req_quest_id, req_state) in &conds.prerequisite_quests {
        if *req_state == 2 {
            if !player.completed_quests.contains(req_quest_id) {
                return CheckResult::Fail;
            }
        }
    }

    CheckResult::Pass
}

pub fn check_completion_conditions(
    conds: &CheckConditions,
    player: &Player,
    active_quest: Option<&crate::player::ActiveQuest>,
) -> CheckResult {
    if conds.has_script {
        return CheckResult::HasScript;
    }

    if let Some(lvmin) = conds.level_min {
        if (player.level as u32) < lvmin {
            return CheckResult::Fail;
        }
    }

    if let Some(lvmax) = conds.level_max {
        if (player.level as u32) > lvmax {
            return CheckResult::Fail;
        }
    }

    if let Some(ref jobs) = conds.job_whitelist {
        let player_job_id = job_to_id(player.job);
        if !jobs.contains(&player_job_id) {
            return CheckResult::Fail;
        }
    }

    if let Some(aq) = active_quest {
        for (mob_id, required) in &conds.required_kills {
            let killed = aq.kill_counts.get(mob_id).copied().unwrap_or(0);
            if killed < *required {
                return CheckResult::Fail;
            }
        }
    } else if !conds.required_kills.is_empty() {
        return CheckResult::Fail;
    }

    CheckResult::Pass
}

pub fn get_start_failed_reason(
    conds: &CheckConditions,
    player: &Player,
) -> Option<&'static str> {
    if let Some(lvmin) = conds.level_min {
        if (player.level as u32) < lvmin {
            return Some("level");
        }
    }
    if let Some(lvmax) = conds.level_max {
        if (player.level as u32) > lvmax {
            return Some("level");
        }
    }
    if let Some(ref jobs) = conds.job_whitelist {
        let player_job_id = job_to_id(player.job);
        if !jobs.contains(&player_job_id) {
            return Some("job");
        }
    }
    for (req_quest_id, req_state) in &conds.prerequisite_quests {
        if *req_state == 2 && !player.completed_quests.contains(req_quest_id) {
            return Some("quest");
        }
    }
    None
}

pub fn get_complete_failed_reason(
    conds: &CheckConditions,
    _player: &Player,
    active_quest: Option<&crate::player::ActiveQuest>,
) -> Option<&'static str> {
    if let Some(aq) = active_quest {
        for (mob_id, required) in &conds.required_kills {
            let killed = aq.kill_counts.get(mob_id).copied().unwrap_or(0);
            if killed < *required {
                return Some("mob");
            }
        }
    }
    for (item_id, count) in &conds.required_items {
        let _ = (item_id, count);
        return Some("item");
    }
    None
}

fn job_to_id(job: Job) -> u32 {
    match job {
        Job::Beginner => 0,
        Job::Warrior => 100,
        Job::Mage => 200,
        Job::Bowman => 300,
        Job::Thief => 400,
        Job::Pirate => 500,
    }
}
