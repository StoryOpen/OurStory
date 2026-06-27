use bevy::prelude::*;
use bevy::text::{EditableText, TextCursorStyle};

use crate::ui::components::*;
use crate::ui::loader::*;

const FRAME_W: f32 = 800.0;
const FRAME_H: f32 = 600.0;
const WINDOW_W: f32 = 1024.0;
const WINDOW_H: f32 = 768.0;

pub fn spawn_login_screen(
    commands: &mut Commands,
    cache: &mut ResMut<WzImageCache>,
    images: &mut ResMut<Assets<Image>>,
) {
    let title_path = "UI/Login.img/Title";
    let common_path = "UI/Login.img/Common";

    let frame = load_ui_sprite(&format!("{common_path}/frame"), cache, images);
    let title = load_ui_sprite(&format!("{title_path}/MSTitle"), cache, images);

    let shadow = load_ui_sprite(&format!("{common_path}/shadow/0"), cache, images);

    let bt_login = load_ui_button(&format!("{title_path}/BtLogin"), cache, images);
    let bt_homepage = load_ui_button(&format!("{title_path}/BtHomePage"), cache, images);
    let bt_new = load_ui_button(&format!("{title_path}/BtNew"), cache, images);
    let bt_quit = load_ui_button(&format!("{title_path}/BtQuit"), cache, images);
    let bt_email_save = load_ui_button(&format!("{title_path}/BtEmailSave"), cache, images);
    let bt_email_lost = load_ui_button(&format!("{title_path}/BtEmailLost"), cache, images);
    let bt_passwd_lost = load_ui_button(&format!("{title_path}/BtPasswdLost"), cache, images);
    let bt_guest = load_ui_button(&format!("{title_path}/BtGuestLogin"), cache, images);
    let _bt_login_id_lost = load_ui_button(&format!("{title_path}/BtLoginIDLost"), cache, images);
    let _bt_login_id_save = load_ui_button(&format!("{title_path}/BtLoginIDSave"), cache, images);

    let check_unchecked = load_ui_sprite(&format!("{title_path}/check/0"), cache, images);
    let check_checked = load_ui_sprite(&format!("{title_path}/check/1"), cache, images);

    let frame_left = (WINDOW_W - FRAME_W) / 2.0;
    let frame_top = (WINDOW_H - FRAME_H) / 2.0;

    commands
        .spawn((
            Name::new("LoginScreen"),
            Node {
                width: Val::Px(WINDOW_W),
                height: Val::Px(WINDOW_H),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            UiLoginScreen,
        ))
        .with_children(|root| {
            if let Some(frame_sprite) = frame {
                root.spawn((
                    Node {
                        width: Val::Px(FRAME_W),
                        height: Val::Px(FRAME_H),
                        position_type: PositionType::Absolute,
                        left: Val::Px(frame_left),
                        top: Val::Px(frame_top),
                        ..default()
                    },
                    ImageNode::from(frame_sprite.handle),
                ));
            }

            if let Some(title_sprite) = title {
                root.spawn((
                    Node {
                        width: Val::Px(397.0),
                        height: Val::Px(219.0),
                        position_type: PositionType::Absolute,
                        left: Val::Px(frame_left + (FRAME_W - 397.0) / 2.0),
                        top: Val::Px(frame_top + 20.0),
                        ..default()
                    },
                    ImageNode::from(title_sprite.handle),
                ));
            }

            let form_left = frame_left + 340.0;
            let form_top = frame_top + 260.0;

            if let Some(shadow_sprite) = &shadow {
                root.spawn((
                    Node {
                        width: Val::Px(133.0),
                        height: Val::Px(71.0),
                        position_type: PositionType::Absolute,
                        left: Val::Px(form_left),
                        top: Val::Px(form_top),
                        ..default()
                    },
                    ImageNode::from(shadow_sprite.handle.clone()),
                ));
            }

            spawn_text_input(
                root,
                form_left + 8.0,
                form_top + 8.0,
                120.0,
                20.0,
                "Email",
                true,
                UiEmailInput,
            );

            let pw_top = form_top + 76.0;
            if let Some(shadow_sprite) = &shadow {
                root.spawn((
                    Node {
                        width: Val::Px(133.0),
                        height: Val::Px(71.0),
                        position_type: PositionType::Absolute,
                        left: Val::Px(form_left),
                        top: Val::Px(pw_top),
                        ..default()
                    },
                    ImageNode::from(shadow_sprite.handle.clone()),
                ));
            }

            spawn_text_input(
                root,
                form_left + 8.0,
                pw_top + 8.0,
                120.0,
                20.0,
                "Password",
                false,
                UiPasswordInput,
            );

            if let Some(btn) = bt_login {
                root.spawn((
                    Node {
                        width: Val::Px(89.0),
                        height: Val::Px(42.0),
                        position_type: PositionType::Absolute,
                        left: Val::Px(form_left + 140.0),
                        top: Val::Px(form_top + 15.0),
                        ..default()
                    },
                    ImageNode::from(btn.normal.clone()),
                    Interaction::default(),
                    UiButton {
                        name: "BtLogin".into(),
                        normal: btn.normal,
                        hover: btn.hover,
                        pressed: btn.pressed,
                        disabled: btn.disabled,
                    },
                ));
            }

            let checkbox_top = form_top + 155.0;

            let check_handle = check_unchecked
                .as_ref()
                .map(|c| c.handle.clone())
                .unwrap_or_default();
            let _check_checked_handle = check_checked
                .as_ref()
                .map(|c| c.handle.clone())
                .unwrap_or(check_handle.clone());

            root.spawn((
                Node {
                    width: Val::Px(18.0),
                    height: Val::Px(23.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(form_left + 10.0),
                    top: Val::Px(checkbox_top),
                    ..default()
                },
                ImageNode::from(check_handle),
                UiLoginCheckbox(false),
            ));

            if let Some(btn) = bt_email_save {
                root.spawn((
                    Node {
                        width: Val::Px(85.0),
                        height: Val::Px(23.0),
                        position_type: PositionType::Absolute,
                        left: Val::Px(form_left + 30.0),
                        top: Val::Px(checkbox_top),
                        ..default()
                    },
                    ImageNode::from(btn.normal.clone()),
                    Interaction::default(),
                    UiButton {
                        name: "BtEmailSave".into(),
                        normal: btn.normal,
                        hover: btn.hover,
                        pressed: btn.pressed,
                        disabled: btn.disabled,
                    },
                ));
            }

            if let Some(btn) = bt_email_lost {
                root.spawn((
                    Node {
                        width: Val::Px(70.0),
                        height: Val::Px(23.0),
                        position_type: PositionType::Absolute,
                        left: Val::Px(form_left + 120.0),
                        top: Val::Px(checkbox_top),
                        ..default()
                    },
                    ImageNode::from(btn.normal.clone()),
                    Interaction::default(),
                    UiButton {
                        name: "BtEmailLost".into(),
                        normal: btn.normal,
                        hover: btn.hover,
                        pressed: btn.pressed,
                        disabled: btn.disabled,
                    },
                ));
            }

            if let Some(btn) = bt_passwd_lost {
                root.spawn((
                    Node {
                        width: Val::Px(66.0),
                        height: Val::Px(23.0),
                        position_type: PositionType::Absolute,
                        left: Val::Px(form_left + 195.0),
                        top: Val::Px(checkbox_top),
                        ..default()
                    },
                    ImageNode::from(btn.normal.clone()),
                    Interaction::default(),
                    UiButton {
                        name: "BtPasswdLost".into(),
                        normal: btn.normal,
                        hover: btn.hover,
                        pressed: btn.pressed,
                        disabled: btn.disabled,
                    },
                ));
            }

            let bottom_y = form_top + 210.0;

            if let Some(btn) = bt_new {
                root.spawn((
                    Node {
                        width: Val::Px(92.0),
                        height: Val::Px(38.0),
                        position_type: PositionType::Absolute,
                        left: Val::Px(form_left + 10.0),
                        top: Val::Px(bottom_y),
                        ..default()
                    },
                    ImageNode::from(btn.normal.clone()),
                    Interaction::default(),
                    UiButton {
                        name: "BtNew".into(),
                        normal: btn.normal,
                        hover: btn.hover,
                        pressed: btn.pressed,
                        disabled: btn.disabled,
                    },
                ));
            }

            if let Some(btn) = bt_homepage {
                root.spawn((
                    Node {
                        width: Val::Px(93.0),
                        height: Val::Px(38.0),
                        position_type: PositionType::Absolute,
                        left: Val::Px(form_left + 110.0),
                        top: Val::Px(bottom_y),
                        ..default()
                    },
                    ImageNode::from(btn.normal.clone()),
                    Interaction::default(),
                    UiButton {
                        name: "BtHomePage".into(),
                        normal: btn.normal,
                        hover: btn.hover,
                        pressed: btn.pressed,
                        disabled: btn.disabled,
                    },
                ));
            }

            if let Some(btn) = bt_quit {
                root.spawn((
                    Node {
                        width: Val::Px(84.0),
                        height: Val::Px(38.0),
                        position_type: PositionType::Absolute,
                        left: Val::Px(form_left + 210.0),
                        top: Val::Px(bottom_y),
                        ..default()
                    },
                    ImageNode::from(btn.normal.clone()),
                    Interaction::default(),
                    UiButton {
                        name: "BtQuit".into(),
                        normal: btn.normal,
                        hover: btn.hover,
                        pressed: btn.pressed,
                        disabled: btn.disabled,
                    },
                ));
            }

            if let Some(btn) = bt_guest {
                root.spawn((
                    Node {
                        width: Val::Px(89.0),
                        height: Val::Px(28.0),
                        position_type: PositionType::Absolute,
                        left: Val::Px(form_left + 110.0),
                        top: Val::Px(bottom_y + 45.0),
                        ..default()
                    },
                    ImageNode::from(btn.normal.clone()),
                    Interaction::default(),
                    UiButton {
                        name: "BtGuestLogin".into(),
                        normal: btn.normal,
                        hover: btn.hover,
                        pressed: btn.pressed,
                        disabled: btn.disabled,
                    },
                ));
            }
        });
}

fn spawn_text_input(
    parent: &mut ChildSpawnerCommands,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    placeholder: &str,
    _is_email: bool,
    marker: impl Component,
) {
    parent
        .spawn((
            Node {
                width: Val::Px(width),
                height: Val::Px(height),
                position_type: PositionType::Absolute,
                left: Val::Px(left),
                top: Val::Px(top),
                padding: UiRect::all(Val::Px(4.0)),
                overflow: Overflow::clip_x(),
                ..default()
            },
            marker,
        ))
        .with_children(|input_parent| {
            input_parent.spawn((
                Text::new(placeholder.to_string()),
                TextFont {
                    font_size: bevy::text::FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgba(0.8, 0.8, 0.8, 0.6)),
                TextLayout::no_wrap(),
                EditableText::default(),
                TextCursorStyle::default(),
            ));
        });
}
