use protocol::types::{DialogBranch, QuestDialog, StopReason};

pub fn start_dialog(quest_id: u32, accept: bool, _failed_reason: Option<&str>) -> QuestDialog {
    let branch = if accept {
        DialogBranch::Yes { pages: 0 }
    } else {
        DialogBranch::No
    };
    QuestDialog {
        quest_id,
        stage: 0,
        branch,
    }
}

pub fn start_stop_dialog(quest_id: u32, reason: &str) -> QuestDialog {
    QuestDialog {
        quest_id,
        stage: 0,
        branch: DialogBranch::Stop {
            reason: reason_to_stop_reason(reason),
        },
    }
}

pub fn complete_dialog(quest_id: u32, completable: bool, failed_reason: Option<&str>) -> QuestDialog {
    if completable {
        QuestDialog {
            quest_id,
            stage: 1,
            branch: DialogBranch::Yes { pages: 0 },
        }
    } else {
        QuestDialog {
            quest_id,
            stage: 1,
            branch: DialogBranch::Stop {
                reason: reason_to_stop_reason(failed_reason.unwrap_or("generic")),
            },
        }
    }
}

fn reason_to_stop_reason(reason: &str) -> StopReason {
    match reason {
        "mob" => StopReason::Mob,
        "item" => StopReason::Item,
        "npc" => StopReason::Npc,
        "quest" => StopReason::Quest,
        _ => StopReason::Generic,
    }
}
