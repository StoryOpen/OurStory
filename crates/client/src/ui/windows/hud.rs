use bevy::prelude::*;

use crate::ui::components::*;
use crate::ui::loader::*;
use crate::wz::get_cached_base;

pub fn spawn_hud(
    commands: &mut Commands,
    cache: &mut ResMut<WzUiSpriteCache>,
    images: &mut ResMut<Assets<Image>>,
) {
    let base = get_cached_base();
    let sb = match base.at_path("UI/StatusBar.img") {
        Ok(n) => n,
        Err(e) => {
            warn!("failed to load StatusBar: {e}");
            return;
        }
    };

    // Load base sprites
    let bg = load_ui_sprite(&sb.at_path("base/backgrnd").unwrap(), cache, images);
    let bg2 = load_ui_sprite(&sb.at_path("base/backgrnd2").unwrap(), cache, images);
    let quickslot_bg = load_ui_sprite(&sb.at_path("base/quickSlot").unwrap(), cache, images);
    let chat_line = load_ui_sprite(&sb.at_path("base/chat").unwrap(), cache, images);
    let chat_target = load_ui_sprite(&sb.at_path("base/chatTarget").unwrap(), cache, images);

    // Load gauges
    let gauge_bar = load_ui_sprite(&sb.at_path("gauge/bar").unwrap(), cache, images);
    let gauge_graduation = load_ui_sprite(&sb.at_path("gauge/graduation").unwrap(), cache, images);
    let gauge_gray = load_ui_sprite(&sb.at_path("gauge/gray").unwrap(), cache, images);

    // Load buttons
    let bt_menu = load_ui_button(&sb.at_path("BtMenu").unwrap(), cache, images);
    let bt_shop = load_ui_button(&sb.at_path("BtShop").unwrap(), cache, images);
    let bt_short = load_ui_button(&sb.at_path("BtShort").unwrap(), cache, images);
    let bt_npt = load_ui_button(&sb.at_path("BtNPT").unwrap(), cache, images);
    let bt_claim = load_ui_button(&sb.at_path("BtClaim").unwrap(), cache, images);
    let bt_whisper = load_ui_button(&sb.at_path("BtWhisper").unwrap(), cache, images);

    // Load key slots
    let stat_key = load_ui_button(&sb.at_path("StatKey").unwrap(), cache, images);
    let equip_key = load_ui_button(&sb.at_path("EquipKey").unwrap(), cache, images);
    let inven_key = load_ui_button(&sb.at_path("InvenKey").unwrap(), cache, images);
    let skill_key = load_ui_button(&sb.at_path("SkillKey").unwrap(), cache, images);

    // HUD layout using flexbox:
    //
    // Root (800x71, anchored bottom):
    //   bg image (absolute, full size)
    //   Main row (flex-row, fill parent):
    //     Left: key buttons row (flex-row, gap 2px)
    //     Center: gauge + menu area (flex-column):
    //       Gauge stack (flex-row): gray bg + bar + graduation
    //       Menu row (flex-row): BtMenu + BtShort + chat target
    //     Right: shop buttons row (flex-row) + quickslot panel

    let hud_entity = commands
        .spawn((
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
            if let Some(bg) = bg {
                parent.spawn((
                    Node {
                        width: Val::Px(800.0),
                        height: Val::Px(71.0),
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        ..default()
                    },
                    ImageNode::from(bg.handle),
                ));
            }

            // Secondary background
            if let Some(bg2) = bg2 {
                parent.spawn((
                    Node {
                        width: Val::Px(570.0),
                        height: Val::Px(71.0),
                        position_type: PositionType::Absolute,
                        left: Val::Px(230.0),
                        top: Val::Px(0.0),
                        ..default()
                    },
                    ImageNode::from(bg2.handle),
                ));
            }

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
                        for (name, btn_data) in [
                            ("StatKey", stat_key),
                            ("EquipKey", equip_key),
                            ("InvenKey", inven_key),
                            ("SkillKey", skill_key),
                        ] {
                            if let Some(btn) = btn_data {
                                keys.spawn((
                                    Node {
                                        width: Val::Px(28.0),
                                        height: Val::Px(20.0),
                                        ..default()
                                    },
                                    ImageNode::from(btn.normal.clone()),
                                    Interaction::default(),
                                    UiButton { name: name.into(), normal: btn.normal, hover: btn.hover, pressed: btn.pressed, disabled: btn.disabled },
                                ));
                            }
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
                        center.spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(2.0),
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|gauge_row| {
                            // Gray background
                            if let Some(gray) = gauge_gray {
                                gauge_row.spawn((
                                    Node { width: Val::Px(340.0), height: Val::Px(16.0), ..default() },
                                    ImageNode::from(gray.handle),
                                ));
                            }
                            // Bar overlay
                            if let Some(bar) = gauge_bar {
                                gauge_row.spawn((
                                    Node {
                                        width: Val::Px(340.0),
                                        height: Val::Px(31.0),
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(0.0),
                                        top: Val::Px(0.0),
                                        ..default()
                                    },
                                    ImageNode::from(bar.handle),
                                ));
                            }
                            // Graduation marks
                            if let Some(grad) = gauge_graduation {
                                gauge_row.spawn((
                                    Node {
                                        width: Val::Px(340.0),
                                        height: Val::Px(31.0),
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(0.0),
                                        top: Val::Px(0.0),
                                        ..default()
                                    },
                                    ImageNode::from(grad.handle),
                                ));
                            }
                        });

                        // Menu row (flex-row)
                        center.spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(2.0),
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|menu_row| {
                            for (name, btn_data) in [
                                ("BtMenu", bt_menu),
                                ("BtShort", bt_short),
                            ] {
                                if let Some(btn) = btn_data {
                                    menu_row.spawn((
                                        Node { width: Val::Px(54.0), height: Val::Px(34.0), ..default() },
                                        ImageNode::from(btn.normal.clone()),
                                        Interaction::default(),
                                        UiButton { name: name.into(), normal: btn.normal, hover: btn.hover, pressed: btn.pressed, disabled: btn.disabled },
                                    ));
                                }
                            }

                            // Chat target
                            if let Some(ct) = chat_target {
                                menu_row.spawn((
                                    Node { width: Val::Px(81.0), height: Val::Px(20.0), ..default() },
                                    ImageNode::from(ct.handle),
                                ));
                            }
                        });

                        // Chat line
                        if let Some(chat) = chat_line {
                            center.spawn((
                                Node { width: Val::Px(566.0), height: Val::Px(5.0), ..default() },
                                ImageNode::from(chat.handle),
                            ));
                        }
                    });

                    // --- RIGHT: Shop buttons + Quickslot (flex-column) ---
                    row.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(2.0),
                        align_items: AlignItems::FlexStart,
                        ..default()
                    })
                    .with_children(|right| {
                        // Shop buttons column
                        right.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(2.0),
                            ..default()
                        })
                        .with_children(|shop_col| {
                            // Top row: Shop + NPT
                            shop_col.spawn(Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(2.0),
                                ..default()
                            })
                            .with_children(|top_row| {
                                for (name, btn_data) in [
                                    ("BtShop", bt_shop),
                                    ("BtNPT", bt_npt),
                                ] {
                                    if let Some(btn) = btn_data {
                                        top_row.spawn((
                                            Node { width: Val::Px(54.0), height: Val::Px(34.0), ..default() },
                                            ImageNode::from(btn.normal.clone()),
                                            Interaction::default(),
                                            UiButton { name: name.into(), normal: btn.normal, hover: btn.hover, pressed: btn.pressed, disabled: btn.disabled },
                                        ));
                                    }
                                }
                            });

                            // Bottom row: Claim + Whisper
                            shop_col.spawn(Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(2.0),
                                ..default()
                            })
                            .with_children(|bot_row| {
                                for (name, btn_data) in [
                                    ("BtClaim", bt_claim),
                                    ("BtWhisper", bt_whisper),
                                ] {
                                    if let Some(btn) = btn_data {
                                        bot_row.spawn((
                                            Node { width: Val::Px(20.0), height: Val::Px(19.0), ..default() },
                                            ImageNode::from(btn.normal.clone()),
                                            Interaction::default(),
                                            UiButton { name: name.into(), normal: btn.normal, hover: btn.hover, pressed: btn.pressed, disabled: btn.disabled },
                                        ));
                                    }
                                }
                            });
                        });

                        // Quickslot panel
                        if let Some(qs) = quickslot_bg {
                            right.spawn((
                                Node { width: Val::Px(151.0), height: Val::Px(80.0), ..default() },
                                ImageNode::from(qs.handle),
                            ));
                        }
                    });
                });
        })
        .id();

    info!("spawned HUD entity: {hud_entity:?}");
}
