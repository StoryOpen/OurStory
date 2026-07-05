use bevy::prelude::*;

use crate::ui::components::*;
use crate::ui::loader::*;
use crate::wz::asset_loaders::WzUiSpriteAsset;

// ── Pending load state ──

#[derive(Resource)]
pub struct PendingStatWindow {
    handles: StatSpriteHandles,
    spawned: bool,
}

struct StatSpriteHandles {
    bg_left: UiSpriteHandle,
    bg_right: UiSpriteHandle,
    basic_stat: UiSpriteHandle,
    bt_auto: ButtonSpriteHandles,
    bt_detail: ButtonSpriteHandles,
}

impl StatSpriteHandles {
    fn load(asset_server: &AssetServer) -> Self {
        let stat_path = "UI/UIWindow.img/Stat";
        Self {
            bg_left: UiSpriteHandle::load(&format!("{stat_path}/backgrnd"), asset_server),
            bg_right: UiSpriteHandle::load(&format!("{stat_path}/backgrnd2"), asset_server),
            basic_stat: UiSpriteHandle::load(&format!("{stat_path}/basicStat"), asset_server),
            bt_auto: ButtonSpriteHandles::load(&format!("{stat_path}/BtAuto"), asset_server),
            bt_detail: ButtonSpriteHandles::load(&format!("{stat_path}/BtDetail"), asset_server),
        }
    }

    fn is_ready(&self, assets: &Assets<WzUiSpriteAsset>) -> bool {
        self.bg_left.is_ready(assets)
            && self.bg_right.is_ready(assets)
            && self.basic_stat.is_ready(assets)
            && self.bt_auto.is_ready(assets)
            && self.bt_detail.is_ready(assets)
    }
}

// ── Entry point ──

pub fn start_stat_load(commands: &mut Commands, asset_server: &AssetServer) {
    let handles = StatSpriteHandles::load(asset_server);
    commands.insert_resource(PendingStatWindow {
        handles,
        spawned: false,
    });
}

// ── Check readiness → spawn ──

pub fn check_stat_ready(
    mut commands: Commands,
    mut pending: Option<ResMut<PendingStatWindow>>,
    sprite_assets: Res<Assets<WzUiSpriteAsset>>,
) {
    let Some(ref mut pending) = pending else { return };
    if pending.spawned {
        return;
    }
    if !pending.handles.is_ready(&sprite_assets) {
        return;
    }

    info!("All stat window assets loaded, spawning");
    spawn_stat_window(&mut commands, &sprite_assets, &pending.handles);
    pending.spawned = true;
}

// ── Spawn (called once assets are ready) ──

fn spawn_stat_window(
    commands: &mut Commands,
    sprite_assets: &Assets<WzUiSpriteAsset>,
    handles: &StatSpriteHandles,
) {
    let bg_left_image = handles.bg_left.image(sprite_assets);
    let bg_right_image = handles.bg_right.image(sprite_assets);
    let basic_stat_image = handles.basic_stat.image(sprite_assets);
    let basic_stat_origin = handles.basic_stat.origin(sprite_assets);
    let bt_auto = handles.bt_auto.to_button(sprite_assets, "BtAuto");
    let bt_detail = handles.bt_detail.to_button(sprite_assets, "BtDetail");

    // Left panel: two-column layout
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
            ImageNode::from(bg_left_image),
            UiWindow {
                name: "Stat".into(),
            },
            UiStatWindow,
        ))
        .with_children(|parent| {
            // Use origin from the resolved asset to position this child sprite
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(-basic_stat_origin.x),
                    top: Val::Px(-basic_stat_origin.y),
                    ..default()
                },
                ImageNode::from(basic_stat_image),
            ));

            // Auto-assign button
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(95.0),
                    top: Val::Px(268.0),
                    width: Val::Px(73.0),
                    height: Val::Px(35.0),
                    ..default()
                },
                ImageNode::from(bt_auto.normal.clone()),
                Interaction::default(),
                bt_auto,
            ));
        });

    // Right panel
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
            ImageNode::from(bg_right_image),
            UiWindow {
                name: "StatDetail".into(),
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(120.0),
                    top: Val::Px(180.0),
                    width: Val::Px(47.0),
                    height: Val::Px(18.0),
                    ..default()
                },
                ImageNode::from(bt_detail.normal.clone()),
                Interaction::default(),
                bt_detail,
            ));
        });
}
