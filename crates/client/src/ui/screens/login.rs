use bevy::prelude::*;
use bevy::text::{EditableText, TextCursorStyle};

use crate::ui::components::*;
use crate::wz::asset_loaders::WzUiBundleAsset;

const FRAME_W: f32 = 800.0;
const FRAME_H: f32 = 600.0;
const WINDOW_W: f32 = 1024.0;
const WINDOW_H: f32 = 768.0;

// ── Pending load state ──

/// All image paths needed by the login screen, hardcoded.
const LOGIN_PATHS: &[&str] = &[
    "UI/Login.img/Common/frame",
    "UI/Login.img/Title/MSTitle",
    "UI/Login.img/Common/shadow/0",
    "UI/Login.img/Title/BtLogin/normal/0",
    "UI/Login.img/Title/BtLogin/mouseOver/0",
    "UI/Login.img/Title/BtLogin/pressed/0",
    "UI/Login.img/Title/BtLogin/disabled/0",
    "UI/Login.img/Title/BtHomePage/normal/0",
    "UI/Login.img/Title/BtHomePage/mouseOver/0",
    "UI/Login.img/Title/BtHomePage/pressed/0",
    "UI/Login.img/Title/BtHomePage/disabled/0",
    "UI/Login.img/Title/BtNew/normal/0",
    "UI/Login.img/Title/BtNew/mouseOver/0",
    "UI/Login.img/Title/BtNew/pressed/0",
    "UI/Login.img/Title/BtNew/disabled/0",
    "UI/Login.img/Title/BtQuit/normal/0",
    "UI/Login.img/Title/BtQuit/mouseOver/0",
    "UI/Login.img/Title/BtQuit/pressed/0",
    "UI/Login.img/Title/BtQuit/disabled/0",
    "UI/Login.img/Title/BtEmailSave/normal/0",
    "UI/Login.img/Title/BtEmailSave/mouseOver/0",
    "UI/Login.img/Title/BtEmailSave/pressed/0",
    "UI/Login.img/Title/BtEmailSave/disabled/0",
    "UI/Login.img/Title/BtEmailLost/normal/0",
    "UI/Login.img/Title/BtEmailLost/mouseOver/0",
    "UI/Login.img/Title/BtEmailLost/pressed/0",
    "UI/Login.img/Title/BtEmailLost/disabled/0",
    "UI/Login.img/Title/BtPasswdLost/normal/0",
    "UI/Login.img/Title/BtPasswdLost/mouseOver/0",
    "UI/Login.img/Title/BtPasswdLost/pressed/0",
    "UI/Login.img/Title/BtPasswdLost/disabled/0",
    "UI/Login.img/Title/BtGuestLogin/normal/0",
    "UI/Login.img/Title/BtGuestLogin/mouseOver/0",
    "UI/Login.img/Title/BtGuestLogin/pressed/0",
    "UI/Login.img/Title/BtGuestLogin/disabled/0",
    "UI/Login.img/Title/BtLoginIDLost/normal/0",
    "UI/Login.img/Title/BtLoginIDLost/mouseOver/0",
    "UI/Login.img/Title/BtLoginIDLost/pressed/0",
    "UI/Login.img/Title/BtLoginIDLost/disabled/0",
    "UI/Login.img/Title/BtLoginIDSave/normal/0",
    "UI/Login.img/Title/BtLoginIDSave/mouseOver/0",
    "UI/Login.img/Title/BtLoginIDSave/pressed/0",
    "UI/Login.img/Title/BtLoginIDSave/disabled/0",
    "UI/Login.img/Title/check/0",
    "UI/Login.img/Title/check/1",
];

fn build_bundle_path() -> String {
    format!("wz://bundle-paths/{}.wzbundle", LOGIN_PATHS.join(","))
}

#[derive(Resource)]
pub struct PendingLoginScreen {
    bundle_handle: Handle<WzUiBundleAsset>,
    spawned: bool,
}

fn image_from_bundle(
    path: &str,
    bundle: &WzUiBundleAsset,
) -> Handle<Image> {
    bundle.images.get(path).cloned().unwrap_or_default()
}

fn origin_from_bundle(
    path: &str,
    bundle: &WzUiBundleAsset,
) -> Vec2 {
    bundle.origins.get(path).copied().unwrap_or(Vec2::ZERO)
}

fn button_from_bundle(
    prefix: &str,
    name: &str,
    bundle: &WzUiBundleAsset,
) -> UiButton {
    UiButton {
        name: name.into(),
        normal: image_from_bundle(&format!("{prefix}/normal/0"), bundle),
        hover: image_from_bundle(&format!("{prefix}/mouseOver/0"), bundle),
        pressed: image_from_bundle(&format!("{prefix}/pressed/0"), bundle),
        disabled: image_from_bundle(&format!("{prefix}/disabled/0"), bundle),
    }
}

// ── Entry point (one bundle load) ──

pub fn start_login_load(commands: &mut Commands, asset_server: &AssetServer) {
    let path = build_bundle_path();
    let bundle_handle = asset_server.load::<WzUiBundleAsset>(&path);
    commands.insert_resource(PendingLoginScreen {
        bundle_handle,
        spawned: false,
    });
}

// ── Check readiness system ──

pub fn check_login_ready(
    mut commands: Commands,
    mut pending: Option<ResMut<PendingLoginScreen>>,
    bundle_assets: Res<Assets<WzUiBundleAsset>>,
) {
    let Some(ref mut pending) = pending else { return };
    if pending.spawned {
        return;
    }
    if !bundle_assets.contains(&pending.bundle_handle) {
        return;
    }

    info!("All login screen assets loaded, spawning UI");
    spawn_login_screen(&mut commands, &bundle_assets, &pending.bundle_handle);
    pending.spawned = true;
}

// ── Spawn (called once assets are ready) ──

fn spawn_login_screen(
    commands: &mut Commands,
    bundle_assets: &Assets<WzUiBundleAsset>,
    bundle_handle: &Handle<WzUiBundleAsset>,
) {
    let Some(bundle) = bundle_assets.get(bundle_handle) else {
        warn!("spawn_login_screen: bundle not ready");
        return;
    };

    let frame_left = (WINDOW_W - FRAME_W) / 2.0;
    let frame_top = (WINDOW_H - FRAME_H) / 2.0;

    let bt_login = button_from_bundle("UI/Login.img/Title/BtLogin", "BtLogin", bundle);
    let bt_homepage = button_from_bundle("UI/Login.img/Title/BtHomePage", "BtHomePage", bundle);
    let bt_new = button_from_bundle("UI/Login.img/Title/BtNew", "BtNew", bundle);
    let bt_quit = button_from_bundle("UI/Login.img/Title/BtQuit", "BtQuit", bundle);
    let bt_email_save = button_from_bundle("UI/Login.img/Title/BtEmailSave", "BtEmailSave", bundle);
    let bt_email_lost = button_from_bundle("UI/Login.img/Title/BtEmailLost", "BtEmailLost", bundle);
    let bt_passwd_lost = button_from_bundle("UI/Login.img/Title/BtPasswdLost", "BtPasswdLost", bundle);
    let bt_guest = button_from_bundle("UI/Login.img/Title/BtGuestLogin", "BtGuestLogin", bundle);

    let frame_image = image_from_bundle("UI/Login.img/Common/frame", bundle);
    let title_image = image_from_bundle("UI/Login.img/Title/MSTitle", bundle);
    let check_unchecked_image = image_from_bundle("UI/Login.img/Title/check/0", bundle);    let shadow_image = image_from_bundle("UI/Login.img/Common/shadow/0", bundle);

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
            root.spawn((
                Node {
                    width: Val::Px(FRAME_W),
                    height: Val::Px(FRAME_H),
                    position_type: PositionType::Absolute,
                    left: Val::Px(frame_left),
                    top: Val::Px(frame_top),
                    ..default()
                },
                ImageNode::from(frame_image),
            ));

            root.spawn((
                Node {
                    width: Val::Px(397.0),
                    height: Val::Px(219.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(frame_left + (FRAME_W - 397.0) / 2.0),
                    top: Val::Px(frame_top + 20.0),
                    ..default()
                },
                ImageNode::from(title_image),
            ));

            let form_left = frame_left + 340.0;
            let form_top = frame_top + 260.0;

            root.spawn((
                Node {
                    width: Val::Px(133.0),
                    height: Val::Px(71.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(form_left),
                    top: Val::Px(form_top),
                    ..default()
                },
                ImageNode::from(shadow_image.clone()),
            ));

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
            root.spawn((
                Node {
                    width: Val::Px(133.0),
                    height: Val::Px(71.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(form_left),
                    top: Val::Px(pw_top),
                    ..default()
                },
                ImageNode::from(shadow_image.clone()),
            ));

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

            root.spawn((
                Node {
                    width: Val::Px(89.0),
                    height: Val::Px(42.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(form_left + 140.0),
                    top: Val::Px(form_top + 15.0),
                    ..default()
                },
                ImageNode::from(bt_login.normal.clone()),
                Interaction::default(),
                bt_login,
            ));

            let checkbox_top = form_top + 155.0;

            root.spawn((
                Node {
                    width: Val::Px(18.0),
                    height: Val::Px(23.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(form_left + 10.0),
                    top: Val::Px(checkbox_top),
                    ..default()
                },
                ImageNode::from(check_unchecked_image),
                UiLoginCheckbox(false),
            ));

            root.spawn((
                Node {
                    width: Val::Px(85.0),
                    height: Val::Px(23.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(form_left + 30.0),
                    top: Val::Px(checkbox_top),
                    ..default()
                },
                ImageNode::from(bt_email_save.normal.clone()),
                Interaction::default(),
                bt_email_save,
            ));

            root.spawn((
                Node {
                    width: Val::Px(70.0),
                    height: Val::Px(23.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(form_left + 120.0),
                    top: Val::Px(checkbox_top),
                    ..default()
                },
                ImageNode::from(bt_email_lost.normal.clone()),
                Interaction::default(),
                bt_email_lost,
            ));

            root.spawn((
                Node {
                    width: Val::Px(66.0),
                    height: Val::Px(23.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(form_left + 195.0),
                    top: Val::Px(checkbox_top),
                    ..default()
                },
                ImageNode::from(bt_passwd_lost.normal.clone()),
                Interaction::default(),
                bt_passwd_lost,
            ));

            let bottom_y = form_top + 210.0;

            root.spawn((
                Node {
                    width: Val::Px(92.0),
                    height: Val::Px(38.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(form_left + 10.0),
                    top: Val::Px(bottom_y),
                    ..default()
                },
                ImageNode::from(bt_new.normal.clone()),
                Interaction::default(),
                bt_new,
            ));

            root.spawn((
                Node {
                    width: Val::Px(93.0),
                    height: Val::Px(38.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(form_left + 110.0),
                    top: Val::Px(bottom_y),
                    ..default()
                },
                ImageNode::from(bt_homepage.normal.clone()),
                Interaction::default(),
                bt_homepage,
            ));

            root.spawn((
                Node {
                    width: Val::Px(84.0),
                    height: Val::Px(42.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(form_left + 210.0),
                    top: Val::Px(bottom_y),
                    ..default()
                },
                ImageNode::from(bt_quit.normal.clone()),
                Interaction::default(),
                bt_quit,
            ));

            root.spawn((
                Node {
                    width: Val::Px(89.0),
                    height: Val::Px(28.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(form_left + 110.0),
                    top: Val::Px(bottom_y + 45.0),
                    ..default()
                },
                ImageNode::from(bt_guest.normal.clone()),
                Interaction::default(),
                bt_guest,
            ));
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
