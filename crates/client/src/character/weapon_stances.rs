//! Weapon-type-specific stance (action) lists.
//!
//! Each weapon type (identified by the first 4 digits of the item ID, e.g. `0130`)
//! supports a specific set of animation actions (stances).
//!
//! ## Structure
//!
//! Stances are split into three tiers:
//!
//! 1. **Universal** — present in the canonical set of **every** weapon type
//!    (`alert`, `fly`, `jump`, `prone`, `proneStab`).  Not listed per-type.
//!
//! 2. **Common variants** — `walk1`/`walk2` and `stand1`/`stand2`.  Every type
//!    that has weapon animations uses one variant or the other (or neither, when
//!    it varies across items).  Tracked in [`COMMON_STANCE_VARIANTS`].
//!
//! 3. **Per-type** — stances that distinguish one weapon category from another.
//!    Listed in [`WEAPON_STANCES`].
//!
//! ## Discovery
//!
//! The canonical set per type is the **intersection** of stances across **all**
//! items of that type (probing `Character/Weapon/<item>.img` with `wz-cli`).
//! This filters out per-item cosmetic extras (`rope`, `ladder`, etc.) so that
//! only the truly type-defining stances remain.

#![allow(dead_code)]

// ---------------------------------------------------------------------------
// Names
// ---------------------------------------------------------------------------

/// Human-readable names derived from the WZ data.
///
/// The name is the generalised **category** name, based on:
/// - The lowest-ID item's name in `String/Eqp.img/Eqp/Weapon/<id>/name`
/// - The `afterImage` and `sfx` fields in `Character/Weapon/<item>.img/info/`
///
/// | Type | WZ item name    | afterImage    | sfx         | Category |
/// |------|-----------------|---------------|-------------|----------|
/// | 0130 | Sword           | swordOL       | swordL      | One-Handed Sword |
/// | 0131 | Double Axe      | swordOL       | swordL      | One-Handed Axe |
/// | 0132 | Mace            | mace          | mace        | One-Handed Blunt |
/// | 0133 | Triangular Zamadar | swordOL    | swordL      | Dagger |
/// | 0137 | Fairy Wand      | mace          | mace        | Wand |
/// | 0138 | Wooden Staff    | mace          | mace        | Staff |
/// | 0139 | *(no String entry)* | barehands | barehands   | *(barehanded fallback)* |
/// | 0140 | Two-Handed Sword  | swordTS     | swordL      | Two-Handed Sword |
/// | 0141 | Two-Handed Axe    | axe         | swordS      | Two-Handed Axe |
/// | 0142 | Wooden Mallet     | axe         | mace        | Two-Handed Blunt |
/// | 0143 | Spear             | spear       | spear       | Spear |
/// | 0144 | Pole Arm          | poleArm     | poleArm     | Polearm |
/// | 0145 | Battle Bow        | bow         | bow         | Bow |
/// | 0146 | Mountain Crossbow | crossBow    | cBow        | Crossbow |
/// | 0147 | Garnier           | swordOL     | tGlove      | Claw |
/// | 0148 | Steel Knuckler    | knuckle     | knuckle     | Knuckle |
/// | 0149 | Pistol            | gun         | gun         | Gun |
/// | 0160 | Basic Skill Effect (warrior) | —   | —          | *(vslot=Ri — not a weapon)* |
/// | 0170 | Dual Plasma Blade | —           | —           | Cannon |
#[rustfmt::skip]
pub const WEAPON_TYPE_NAMES: &[(&str, &str)] = &[
    ("0130", "One-Handed Sword"),
    ("0131", "One-Handed Axe"),
    ("0132", "One-Handed Blunt"),
    ("0133", "Dagger"),
    ("0137", "Wand"),
    ("0138", "Staff"),
    ("0139", "Barehanded (single item 01392000, afterImage=barehands)"),
    ("0140", "Two-Handed Sword"),
    ("0141", "Two-Handed Axe"),
    ("0142", "Two-Handed Blunt"),
    ("0143", "Spear"),
    ("0144", "Polearm"),
    ("0145", "Bow"),
    ("0146", "Crossbow"),
    ("0147", "Claw"),
    ("0148", "Knuckle"),
    ("0149", "Gun"),
    ("0160", "Skill Effect (vslot=Ri — not a weapon)"),
    ("0170", "Cannon"),
];

// ---------------------------------------------------------------------------
// Stance tiers
// ---------------------------------------------------------------------------

/// Stances present in the canonical set of **every** weapon type that has
/// weapon animations.  Not listed in [`WEAPON_STANCES`].
pub const UNIVERSAL_STANCES: &[&str] = &["alert", "fly", "jump", "prone", "proneStab"];

/// Walk/stand variant used by each weapon type.
///
/// Some types (0142, 0144) don't have a consistent walk or stand variant
/// across items — they are absent from this map.
#[rustfmt::skip]
pub const COMMON_STANCE_VARIANTS: &[(&str, &[&str])] = &[
    // walk variant, stand variant
    ("0130", &["walk1", "stand1"]),
    ("0131", &["walk1", "stand1"]),
    ("0132", &["walk1", "stand1"]),
    ("0133", &["walk1", "stand1"]),
    ("0137", &["walk1", "stand1"]),
    ("0138", &["walk1", "stand1"]),
    ("0139", &["walk1", "stand1"]),
    ("0140", &["walk1", "stand2"]),
    ("0141", &["walk2", "stand2"]),
    ("0142", &[/* walk varies */     "stand2"]),
    ("0143", &["walk2", "stand2"]),
    ("0144", &[/* walk varies */     /* stand varies */]),
    ("0145", &["walk1", "stand1"]),
    ("0146", &["walk2", "stand2"]),
    ("0147", &["walk1", "stand1"]),
    ("0148", &["walk1", "stand1"]),
    ("0149", &["walk1", "stand1"]),
];

// ---------------------------------------------------------------------------
// Per-type unique stances
// ---------------------------------------------------------------------------
//
// These lists exclude UNIVERSAL_STANCES and COMMON_STANCE_VARIANTS.
// Derived by intersecting ALL items of each type in Character.wz.

/// Shared by types 0130, 0131, 0132, 0133, 0147 (one-handed melee + Claw).
const STANCES_1H_MELEE: &[&str] = &[
    "heal", "stabO1", "stabO2", "stabOF", "swingO1", "swingO2", "swingO3",
    "swingOF",
];

/// Shared by types 0137, 0138 (Wand, Staff).
const STANCES_MAGIC: &[&str] = &[
    "heal", "shoot1", "shootF", "stabO1", "stabO2", "swingO1", "swingO2",
    "swingO3",
];

/// Type 0139 (Two-Handed Sword, single item 01392000.img — superset).
const STANCES_2H_SWORD: &[&str] = &[
    "heal", "shoot1", "shoot2", "shootF", "stabO1", "stabO2", "stabOF",
    "stabT1", "stabT2", "stabTF", "swingO2", "swingO3", "swingOF", "swingP1",
    "swingP2", "swingPF", "swingT1", "swingT2", "swingT3", "swingTF",
];

/// Type 0140 (Two-Handed Sword).
const STANCES_2H_AXE: &[&str] = &[
    "stabO1", "stabO2", "stabOF", "swingT1", "swingT2", "swingT3", "swingTF",
];

/// Types 0141, 0142 (Two-Handed Axe, Two-Handed Blunt).
const STANCES_2H_BLUNT: &[&str] = &[
    "stabO1", "stabO2", "stabOF", "swingT1", "swingT2", "swingT3",
];

/// Types 0143, 0144 (Spear, Polearm).
const STANCES_POLEARM: &[&str] = &[
    "stabT1", "stabT2", "stabTF", "swingP1", "swingP2", "swingPF", "swingT2",
];

/// Type 0145 (Bow).
const STANCES_BOW: &[&str] = &["shoot1", "shootF", "swingT1", "swingT3"];

/// Type 0146 (Crossbow).
const STANCES_CROSSBOW: &[&str] = &["shoot2", "stabT1", "swingT1"];

/// Type 0148 (Knuckle).
const STANCES_KNUCKLE: &[&str] = &[
    "heal", "shoot2", "sit", "stabO1", "stabO2", "stabOF", "stabT2", "stabTF",
    "swingO2", "swingOF", "swingP2", "swingPF", "swingT1", "swingT2",
];

/// Type 0149 (Gun).
const STANCES_GUN: &[&str] = &[
    "shoot2", "stabO1", "stabO2", "stabT2", "swingO3", "swingP1", "swingP2",
    "swingT1", "swingT2", "swingT3",
];

// ---------------------------------------------------------------------------
// Main lookup
// ---------------------------------------------------------------------------

/// Map from 4‑digit weapon‑type code to the list of **distinctive** stances
/// (universal stances and common walk/stand variants are **not** included).
///
/// Weapon types without weapon‑specific animations (`0160`, `0170`) have an
/// empty slice.
pub const WEAPON_STANCES: &[(&str, &[&str])] = &[
    // ── One-handed melee ──────────────────────────────────────────────
    ("0130", STANCES_1H_MELEE),
    ("0131", STANCES_1H_MELEE),
    ("0132", STANCES_1H_MELEE),
    ("0133", STANCES_1H_MELEE),
    // ── Magic ─────────────────────────────────────────────────────────
    ("0137", STANCES_MAGIC),
    ("0138", STANCES_MAGIC),
    // ── Two-Handed Sword (single item 01392000.img) ───────────────────
    ("0139", STANCES_2H_SWORD),
    // ── Two-handed melee ──────────────────────────────────────────────
    ("0140", STANCES_2H_AXE),
    ("0141", STANCES_2H_BLUNT),
    ("0142", STANCES_2H_BLUNT),
    // ── Polearm / Spear ───────────────────────────────────────────────
    ("0143", STANCES_POLEARM),
    ("0144", STANCES_POLEARM),
    // ── Ranged ────────────────────────────────────────────────────────
    ("0145", STANCES_BOW),
    ("0146", STANCES_CROSSBOW),
    // ── Claw (same as 1H melee) ───────────────────────────────────────
    ("0147", STANCES_1H_MELEE),
    // ── Pirate ────────────────────────────────────────────────────────
    ("0148", STANCES_KNUCKLE),
    ("0149", STANCES_GUN),
    // ── No weapon-specific stances ────────────────────────────────────
    ("0160", &[]),
    ("0170", &[]),
];

// ---------------------------------------------------------------------------
// Convenience helpers
// ---------------------------------------------------------------------------

/// Look up the distinctive stances for a weapon type by its 4‑digit code.
///
/// Returns `None` if the code is unknown.
pub fn stances_for_weapon_type(code: &str) -> Option<&'static [&'static str]> {
    WEAPON_STANCES
        .iter()
        .find(|(k, _)| *k == code)
        .map(|(_, v)| *v)
}
