# Quest System — Server-Core

## Architecture Decisions

### Server authoritative, client presentational

All quest logic (validation, state transitions, rewards, item transactions) runs on the server. The client never interprets conditions or trusts player input for quest state. This prevents cheating and ensures consistent state across all clients.

### Wire protocol carries IDs only

Server sends quest IDs and numbers (counts, reward amounts). Client resolves display text locally from WZ files (`QuestInfo.img` for names/descriptions, `Say.img` for dialog). This avoids sending verbose dialog strings over the network since the client already loads WZ assets for rendering.

### Server picks dialog branch

`Say.img` has branching dialog (yes/no/stop). The server evaluates `Check.img` conditions and tells the client which branch to show via `DialogBranch`. The client just renders the text for that branch — it never evaluates conditions to decide which dialog to display.

### Two-phase check/act model

Each quest has separate start conditions (`Check/0`) and completion conditions (`Check/1`), with matching actions (`Act/0` for start, `Act/1` for completion). This cleanly separates "can I accept this quest?" from "have I finished it?"

### Scripted quests deferred

~53 quests reference `startscript`/`endscript` Lua scripts (job advancement, teleportation, minigames). These are recognized but not executable until a Lua VM is integrated. The server returns `ScriptedQuest` error when these are attempted.

---

## Data Flow

```
WZ Files (Quest.wz)
  │
  ├─ Server loads: Check.img, Act.img, PQuest.img, Exclusive.img
  │  → QuestRegistry (HashMap<u32, QuestDef>)
  │  → Evaluates conditions, applies rewards
  │
  └─ Client loads: QuestInfo.img, Say.img
     → Quest text cache (names, descriptions, dialog)
     → Renders text by quest ID + stage + branch
```

### Packet Flow

```
Client                          Server
  │                               │
  │── NpcQuest(npc_id) ─────────>│  Query available starts/completions
  │<── NpcQuestList(ids) ────────│  Server sends quest IDs only
  │                               │
  │── QuestStart(id, accept) ───>│  Validate Check/0, apply Act/0
  │<── QuestStarted(id, objs) ───│  Return objectives + dialog branch
  │                               │
  │   ... player kills mobs ...   │
  │<── QuestProgress(id, 30/43) ─│  Push progress updates
  │                               │
  │── QuestComplete(id) ────────>│  Validate Check/1, apply Act/1
  │<── QuestCompleted(rewards) ──│  Return EXP, items, next quest
```

---

## WZ Data Mapping

### QuestInfo.img → Client display

| Field | Usage |
|---|---|
| `name` | Quest display name |
| `0`, `1`, `2` | Pre-start, active, complete description text |
| `area` | Region/zone ID for grouping |
| `parent` | Display name of parent quest chain |
| `order` | Ordering within chain |
| `autoStart` | Auto-starts when entering map |
| `autoComplete` | Auto-completes when conditions met |
| `demandSummary` | Summary of requirements (markup) |
| `rewardSummary` | Summary of rewards (markup) |

### Check.img → Server validation

| Field | Maps to |
|---|---|
| `npc` | `CheckConditions.npc_id` |
| `lvmin`/`lvmax` | `CheckConditions.level_min/level_max` |
| `job` | `CheckConditions.job_whitelist` |
| `quest` | `CheckConditions.prerequisite_quests` |
| `item` | `CheckConditions.required_items` |
| `mob` | `CheckConditions.required_kills` |
| `skill` | `CheckConditions.required_skills` |
| `interval` | `CheckConditions.cooldown_minutes` |
| `start`/`end` | `CheckConditions.time_start/time_end` |
| `normalAutoStart` | `CheckConditions.normal_auto_start` |
| `startscript`/`endscript` | `QuestDef.start_script/end_script` |

### Act.img → Server rewards

| Field | Maps to |
|---|---|
| `exp` | `QuestActions.exp` |
| `item` | `QuestActions.items` (positive = give, negative = take) |
| `item.period` | `ItemAction.period_minutes` |
| `item.job` | `ItemAction.job_filter` |
| `nextQuest` | `QuestActions.next_quest` |
| `npcAct` | `QuestActions.npc_act` |
| `skill` | `QuestActions.skill_grants` |
| `petspeed` | `QuestActions.pet_speed` |

### Say.img → Client dialog (resolved by server branch selection)

```
Say.img/<questId>/
  0/   ← start dialog
  1/   ← complete dialog

  Each stage:
    0       = "Base dialog text"
    yes/    = {0: "text", 1: "text", ...}  (multi-page accept)
    no/     = {0: "text"}                   (decline)
    stop/   = conditions that failed:
      stop/mob/0   = "You haven't killed enough..."
      stop/item/0  = "You don't have the items..."
      stop/npc/0   = "Go talk to..."
```

---

## Key Types

```rust
// Server-side quest definition (loaded from WZ)
struct QuestDef {
    id, name, area, auto_start, auto_complete,
    start_check: CheckConditions,
    complete_check: CheckConditions,
    start_act: QuestActions,
    complete_act: QuestActions,
    start_script: Option<String>,  // deferred
    end_script: Option<String>,    // deferred
}

// Player quest state (on Player struct)
struct ActiveQuest {
    quest_id: u32,
    kill_counts: HashMap<u32, u32>,  // mob_id -> count
    started_at: Instant,
}

// Wire protocol types (protocol crate)
enum DialogBranch { Yes { pages }, No, Stop { reason: StopReason } }
enum StopReason { Mob, Item, Npc, Quest, Generic }
```

---

## Completed

- [x] Quest data types (`QuestState`, `ObjectiveType`, `ObjectiveInfo`, `ItemGrant`, `StopReason`, `DialogBranch`, `QuestDialog`)
- [x] Quest packet structs (11 packets: request/response for NPC talk, start, complete, forfeit, progress, sync)
- [x] Quest opcodes (4 recv + 6 send)
- [x] Player quest state (`active_quests`, `completed_quests` on `Player`)
- [x] Quest definition types (`QuestDef`, `CheckConditions`, `QuestActions`, `ItemAction`, `SkillGrant`)
- [x] WZ loader — parses `QuestInfo.img`, `Check.img`, `Act.img` into `QuestDef` structs
- [x] Condition evaluator — validates level, job, prerequisites, mob kills, items, skills, time windows
- [x] Dialog resolver — server picks yes/no/stop branch with reason
- [x] QuestRegistry API — `load`, `get`, `available_starts`, `available_completions`, `start_quest`, `complete_quest`, `forfeit_quest`, `on_mob_killed`, `get_objectives`, `get_dialog`
- [x] Mob kill tracking — increments `kill_counts` on `ActiveQuest`, returns `QuestProgressUpdate`
- [x] Full workspace compiles cleanly

---

## Needs Implementation

### High priority

- [ ] **QuestRegistry integration** — Wire `QuestRegistry` into `World` struct so it's accessible from channel/map handlers
- [ ] **Packet handlers** — Handle `NpcQuest`, `QuestStart`, `QuestComplete`, `QuestForfeit` recv opcodes in the server binary (requires TCP accept + packet routing to be wired up first)

### Medium priority

- [ ] **DB schema** — `quest_progress` table (player_id, quest_id, state, kill_data JSON, timestamps)
- [ ] **DB persistence** — Extend `Database` trait with `load_quests`, `save_quest`, `complete_quest`
- [ ] **Mob kill → quest hook** — When a mob dies in `Map`, call `QuestRegistry::on_mob_killed` and push `QuestProgress` packets
- [ ] **autoStart/autoComplete** — Check and auto-transition on map enter and after mob kills
- [ ] **Repeatable quest cooldowns** — Track `interval` per quest, enforce cooldown before re-accept
- [ ] **Item inventory integration** — Actual item give/take in `start_quest`/`complete_quest` (currently computes but doesn't apply)

### Low priority

- [ ] **Scripted quest support** — Lua VM integration for `startscript`/`endscript` quests
- [ ] **Client QuestPlugin** — Bevy plugin for rendering quest indicators, dialog UI, tracker HUD
- [ ] **Client WZ text loading** — Load `QuestInfo.img` and `Say.img` into client-side cache
- [ ] **Quest chain display** — Group quests by `parent`/`order` in quest log UI
- [ ] **Time-limited quests** — Enforce `start`/`end` datetime windows
- [ ] **PQuest integration** — Party quest specific logic (PQuest.img data loaded but not used)
- [ ] **Exclusive medal tracking** — Flag quests that award exclusive medals (Exclusive.img loaded but not used)
