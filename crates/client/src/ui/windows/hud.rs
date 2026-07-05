use bevy::prelude::*;

use crate::ui::components::*;
use crate::ui::loader::*;
use crate::wz::asset_loaders::WzUiSpriteAsset;

// ── Pending load state ──

#[derive(Resource)]
pub struct PendingHud {
    handles: HudSpriteHandles,
    spawned: bool,
}

struct HudSpriteHandles {
    bg: UiSpriteHandle,
    bg2: UiSpriteHandle,
    quickslot_bg: UiSpriteHandle,
    chat_line: UiSpriteHandle,
    chat_target: UiSpriteHandle,
    gauge_bar: UiSpriteHandle,
    gauge_graduation: UiSpriteHandle,
    gauge_gray: UiSpriteHandle,
    bt_menu: ButtonSpriteHandles,
    bt_shop: ButtonSpriteHandles,
    bt_short: ButtonSpriteHandles,
    bt_npt: ButtonSpriteHandles,
    bt_claim: ButtonSpriteHandles,
    bt_whisper: ButtonSpriteHandles,
    stat_key: ButtonSpriteHandles,
    equip_key: ButtonSpriteHandles,
    inven_key: ButtonSpriteHandles,
    skill_key: ButtonSpriteHandles,
}

impl HudSpriteHandles {
    fn load(asset_server: &AssetServer) -> Self {
        let sb_path = "UI/StatusBar.img";
        Self {
            bg: UiSpriteHandle::load(&format!("{sb_path}/base/backgrnd"), asset_server),
            bg2: UiSpriteHandle::load(&format!("{sb_path}/base/backgrnd2"), asset_server),
            quickslot_bg: UiSpriteHandle::load(&format!("{sb_path}/base/quickSlot"), asset_server),
            chat_line: UiSpriteHandle::load(&format!("{sb_path}/base/chat"), asset_server),
            chat_target: UiSpriteHandle::load(&format!("{sb_path}/base/chatTarget"), asset_server),
            gauge_bar: UiSpriteHandle::load(&format!("{sb_path}/gauge/bar"), asset_server),
            gauge_graduation: UiSpriteHandle::load(&format!("{sb_path}/gauge/graduation"), asset_server),
            gauge_gray: UiSpriteHandle::load(&format!("{sb_path}/gauge/gray"), asset_server),
            bt_menu: ButtonSpriteHandles::load(&format!("{sb_path}/BtMenu"), asset_server),
            bt_shop: ButtonSpriteHandles::load(&format!("{sb_path}/BtShop"), asset_server),
            bt_short: ButtonSpriteHandles::load(&format!("{sb_path}/BtShort"), asset_server),
            bt_npt: ButtonSpriteHandles::load(&format!("{sb_path}/BtNPT"), asset_server),
            bt_claim: ButtonSpriteHandles::load(&format!("{sb_path}/BtClaim"), asset_server),
            bt_whisper: ButtonSpriteHandles::load(&format!("{sb_path}/BtWhisper"), asset_server),
            stat_key: ButtonSpriteHandles::load(&format!("{sb_path}/StatKey"), asset_server),
            equip_key: ButtonSpriteHandles::load(&format!("{sb_path}/EquipKey"), asset_server),
            inven_key: ButtonSpriteHandles::load(&format!("{sb_path}/InvenKey"), asset_server),
            skill_key: ButtonSpriteHandles::load(&format!("{sb_path}/SkillKey"), asset_server),
        }
    }

    fn is_ready(&self, assets: &Assets<WzUiSpriteAsset>) -> bool {
        self.bg.is_ready(assets)
            && self.bg2.is_ready(assets)
            && self.quickslot_bg.is_ready(assets)
            && self.chat_line.is_ready(assets)
            && self.chat_target.is_ready(assets)
            && self.gauge_bar.is_ready(assets)
            && self.gauge_graduation.is_ready(assets)
            && self.gauge_gray.is_ready(assets)
            && self.bt_menu.is_ready(assets)
            && self.bt_shop.is_ready(assets)
            && self.bt_short.is_ready(assets)
            && self.bt_npt.is_ready(assets)
            && self.bt_claim.is_ready(assets)
            && self.bt_whisper.is_ready(assets)
            && self.stat_key.is_ready(assets)
            && self.equip_key.is_ready(assets)
            && self.inven_key.is_ready(assets)
            && self.skill_key.is_ready(assets)
    }
}

// ── Entry point ──

pub fn start_hud_load(commands: &mut Commands, asset_server: &AssetServer) {
    let handles = HudSpriteHandles::load(asset_server);
    commands.insert_resource(PendingHud {
        handles,
        spawned: false,
    });
}

// ── Check readiness → spawn ──

pub fn check_hud_ready(
    mut commands: Commands,
    mut pending: Option<ResMut<PendingHud>>,
    sprite_assets: Res<Assets<WzUiSpriteAsset>>,
) {
    let Some(ref mut pending) = pending else { return };
    if pending.spawned {
        return;
    }
    if !pending.handles.is_ready(&sprite_assets) {
        return;
    }

    info!("All HUD assets loaded, spawning");
    spawn_hud(&mut commands, &sprite_assets, &pending.handles);
    pending.spawned = true;
}

// ── Spawn (called once assets are ready) ──

fn spawn_hud(
    commands: &mut Commands,
    sprite_assets: &Assets<WzUiSpriteAsset>,
    handles: &HudSpriteHandles,
) {
    let bg_image = handles.bg.image(sprite_assets);
    let bg2_image = handles.bg2.image(sprite_assets);
    let quickslot_bg_image = handles.quickslot_bg.image(sprite_assets);
    let chat_line_image = handles.chat_line.image(sprite_assets);
    let chat_target_image = handles.chat_target.image(sprite_assets);
    let gauge_bar_image = handles.gauge_bar.image(sprite_assets);
    let gauge_graduation_image = handles.gauge_graduation.image(sprite_assets);
    let gauge_gray_image = handles.gauge_gray.image(sprite_assets);

    let bt_menu = handles.bt_menu.to_button(sprite_assets, "BtMenu");
    let bt_shop = handles.bt_shop.to_button(sprite_assets, "BtShop");
    let bt_short = handles.bt_short.to_button(sprite_assets, "BtShort");
    let bt_npt = handles.bt_npt.to_button(sprite_assets, "BtNPT");
    let bt_claim = handles.bt_claim.to_button(sprite_assets, "BtClaim");
    let bt_whisper = handles.bt_whisper.to_button(sprite_assets, "BtWhisper");
    let stat_key = handles.stat_key.to_button(sprite_assets, "StatKey");
    let equip_key = handles.equip_key.to_button(sprite_assets, "EquipKey");
    let inven_key = handles.inven_key.to_button(sprite_assets, "InvenKey");
    let skill_key = handles.skill_key.to_button(sprite_assets, "SkillKey");

    commands
        .spawn((
            Name::new("HUD"),
            Node {
                width: Val::Px(800.0),
                height: Val::Px(71.0),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                bottom: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            UiHud,
        ))
        .with_children(|parent| {
            // Background image (absolute, behind everything)
            parent.spawn((
                Node {
                    width: Val::Px(800.0),
                    height: Val::Px(71.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    ..default()
                },
                ImageNode::from(bg_image),
            ));

            // Secondary background
            parent.spawn((
                Node {
                    width: Val::Px(570.0),
                    height: Val::Px(71.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(230.0),
                    top: Val::Px(0.0),
                    ..default()
                },
                ImageNode::from(bg2_image),
            ));

            // === MAIN CONTENT ROW (flex-row) ===
            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(71.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    padding: UiRect::new(Val::Px(6.0), Val::Px(6.0), Val::Px(0.0), Val::Px(0.0)),
                    column_gap: Val::Px(4.0),
                    ..default()
                })
                .with_children(|row| {
                    // --- LEFT: Key menu buttons (flex-row) ---
                    row.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(2.0),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|keys| {
                        for btn in [&stat_key, &equip_key, &inven_key, &skill_key] {
                            keys.spawn((
                                Node {
                                    width: Val::Px(28.0),
                                    height: Val::Px(20.0),
                                    ..default()
                                },
                                ImageNode::from(btn.normal.clone()),
                                Interaction::default(),
                                UiButton {
                                    name: btn.name.clone(),
                                    normal: btn.normal.clone(),
                                    hover: btn.hover.clone(),
                                    pressed: btn.pressed.clone(),
                                    disabled: btn.disabled.clone(),
                                },
                            ));
                        }
                    });

                    // --- CENTER: Gauge + Menu area (flex-column, grow) ---
                    row.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        flex_grow: 1.0,
                        row_gap: Val::Px(2.0),
                        align_items: AlignItems::FlexStart,
                        ..default()
                    })
                    .with_children(|center| {
                        // Gauge row (flex-row)
                        center
                            .spawn(Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(2.0),
                                align_items: AlignItems::Center,
                                ..default()
                            })
                            .with_children(|gauge_row| {
                                gauge_row.spawn((
                                    Node {
                                        width: Val::Px(340.0),
                                        height: Val::Px(16.0),
                                        ..default()
                                    },
                                    ImageNode::from(gauge_gray_image.clone()),
                                ));
                                gauge_row.spawn((
                                    Node {
                                        width: Val::Px(340.0),
                                        height: Val::Px(31.0),
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(0.0),
                                        top: Val::Px(0.0),
                                        ..default()
                                    },
                                    ImageNode::from(gauge_bar_image.clone()),
                                ));
                                gauge_row.spawn((
                                    Node {
                                        width: Val::Px(340.0),
                                        height: Val::Px(31.0),
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(0.0),
                                        top: Val::Px(0.0),
                                        ..default()
                                    },
                                    ImageNode::from(gauge_graduation_image.clone()),
                                ));
                            });

                        // Menu row (flex-row)
                        center
                            .spawn(Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(2.0),
                                align_items: AlignItems::Center,
                                ..default()
                            })
                            .with_children(|menu_row| {
                                for btn in [&bt_menu, &bt_short] {
                                    menu_row.spawn((
                                        Node {
                                            width: Val::Px(54.0),
                                            height: Val::Px(34.0),
                                            ..default()
                                        },
                                        ImageNode::from(btn.normal.clone()),
                                        Interaction::default(),
                                        UiButton {
                                            name: btn.name.clone(),
                                            normal: btn.normal.clone(),
                                            hover: btn.hover.clone(),
                                            pressed: btn.pressed.clone(),
                                            disabled: btn.disabled.clone(),
                                        },
                                    ));
                                }

                                menu_row.spawn((
                                    Node {
                                        width: Val::Px(81.0),
                                        height: Val::Px(20.0),
                                        ..default()
                                    },
                                    ImageNode::from(chat_target_image.clone()),
                                ));
                            });

                        // Chat line
                        center.spawn((
                            Node {
                                width: Val::Px(566.0),
                                height: Val::Px(5.0),
                                ..default()
                            },
                            ImageNode::from(chat_line_image.clone()),
                        ));
                    });

                    // --- RIGHT: Shop buttons + Quickslot (flex-row) ---
                    row.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(2.0),
                        align_items: AlignItems::FlexStart,
                        ..default()
                    })
                    .with_children(|right| {
                        // Shop buttons column
                        right
                            .spawn(Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(2.0),
                                ..default()
                            })
                            .with_children(|shop_col| {
                                shop_col
                                    .spawn(Node {
                                        flex_direction: FlexDirection::Row,
                                        column_gap: Val::Px(2.0),
                                        ..default()
                                    })
                                    .with_children(|top_row| {
                                        for btn in [&bt_shop, &bt_npt] {
                                            top_row.spawn((
                                                Node {
                                                    width: Val::Px(54.0),
                                                    height: Val::Px(34.0),
                                                    ..default()
                                                },
                                                ImageNode::from(btn.normal.clone()),
                                                Interaction::default(),
                                                UiButton {
                                                    name: btn.name.clone(),
                                                    normal: btn.normal.clone(),
                                                    hover: btn.hover.clone(),
                                                    pressed: btn.pressed.clone(),
                                                    disabled: btn.disabled.clone(),
                                                },
                                            ));
                                        }
                                    });

                                shop_col
                                    .spawn(Node {
                                        flex_direction: FlexDirection::Row,
                                        column_gap: Val::Px(2.0),
                                        ..default()
                                    })
                                    .with_children(|bot_row| {
                                        for btn in [&bt_claim, &bt_whisper] {
                                            bot_row.spawn((
                                                Node {
                                                    width: Val::Px(20.0),
                                                    height: Val::Px(19.0),
                                                    ..default()
                                                },
                                                ImageNode::from(btn.normal.clone()),
                                                Interaction::default(),
                                                UiButton {
                                                    name: btn.name.clone(),
                                                    normal: btn.normal.clone(),
                                                    hover: btn.hover.clone(),
                                                    pressed: btn.pressed.clone(),
                                                    disabled: btn.disabled.clone(),
                                                },
                                            ));
                                        }
                                    });
                            });

                        // Quickslot panel
                        right.spawn((
                            Node {
                                width: Val::Px(151.0),
                                height: Val::Px(80.0),
                                ..default()
                            },
                            ImageNode::from(quickslot_bg_image),
                        ));
                    });
                });
        });
}
