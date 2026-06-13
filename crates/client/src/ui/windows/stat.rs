use bevy::prelude::*;

use crate::ui::components::*;
use crate::ui::loader::*;

pub fn spawn_stat_window(
    commands: &mut Commands,
    cache: &mut ResMut<WzImageCache>,
    images: &mut ResMut<Assets<Image>>,
) {
    let stat_path = "UI/UIWindow.img/Stat";

    let bg_left = match load_ui_sprite(&format!("{stat_path}/backgrnd"), cache, images) {
        Some(s) => s,
        None => return,
    };
    let bg_right = load_ui_sprite(&format!("{stat_path}/backgrnd2"), cache, images);
    let basic_stat = load_ui_sprite(&format!("{stat_path}/basicStat"), cache, images);
    let bt_auto = load_ui_button(&format!("{stat_path}/BtAuto"), cache, images);
    let bt_detail = load_ui_button(&format!("{stat_path}/BtDetail"), cache, images);

    // Left panel: two-column layout
    // Column 1 (left): stat labels (STR/DEX/INT/LUK) stacked vertically
    // Column 2 (right): background + buttons
    commands
        .spawn((
            Name::new("StatWindow"),
            Node {
                width: Val::Px(175.0),
                height: Val::Px(347.0),
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                top: Val::Px(20.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ImageNode::from(bg_left.handle),
            UiWindow {
                name: "Stat".into(),
            },
            UiStatWindow,
        ))
        .with_children(|parent| {
            if let Some(bs) = basic_stat {
                parent.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(-bs.origin.x),
                        top: Val::Px(-bs.origin.y),
                        ..default()
                    },
                    ImageNode::from(bs.handle),
                ));
            }

            // Auto-assign button — bottom area
            if let Some(btn) = bt_auto {
                parent.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(95.0),
                        top: Val::Px(268.0),
                        width: Val::Px(73.0),
                        height: Val::Px(35.0),
                        ..default()
                    },
                    ImageNode::from(btn.normal.clone()),
                    Interaction::default(),
                    UiButton {
                        name: "BtAuto".into(),
                        normal: btn.normal,
                        hover: btn.hover,
                        pressed: btn.pressed,
                        disabled: btn.disabled,
                    },
                ));
            }
        });

    // Right panel
    if let Some(right_bg) = bg_right {
        commands
            .spawn((
                Node {
                    width: Val::Px(177.0),
                    height: Val::Px(203.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(200.0),
                    top: Val::Px(164.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                ImageNode::from(right_bg.handle),
                UiWindow {
                    name: "StatDetail".into(),
                },
            ))
            .with_children(|parent| {
                if let Some(btn) = bt_detail {
                    parent.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(120.0),
                            top: Val::Px(180.0),
                            width: Val::Px(47.0),
                            height: Val::Px(18.0),
                            ..default()
                        },
                        ImageNode::from(btn.normal.clone()),
                        Interaction::default(),
                        UiButton {
                            name: "BtDetail".into(),
                            normal: btn.normal,
                            hover: btn.hover,
                            pressed: btn.pressed,
                            disabled: btn.disabled,
                        },
                    ));
                }
            });
    }
}
