use super::resources::{BaseResolution, MainCamera};
use crate::input::IsLocalPlayer;
use crate::map::events::MapLoaded;
use crate::map::resources::MapBounds;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::ui::UiScale;

pub fn reset_camera(
    trigger: On<MapLoaded>,
    mut camera: Query<(&mut Transform, &Projection), With<MainCamera>>,
) {
    let Ok((mut transform, projection)) = camera.single_mut() else {
        return;
    };
    let Projection::Orthographic(projection) = projection else {
        return;
    };
    let half_h = projection.area.height() * 0.5;
    transform.translation.x = trigger.event().bounds.center().x;
    transform.translation.y = -half_h;
}

pub fn follow_player(
    player: Query<&Transform, (With<IsLocalPlayer>, Without<MainCamera>)>,
    mut camera: Query<&mut Transform, With<MainCamera>>,
) {
    let Ok(player_tf) = player.single() else {
        return;
    };
    let Ok(mut camera_tf) = camera.single_mut() else {
        return;
    };
    camera_tf.translation.x = player_tf.translation.x;
    camera_tf.translation.y = player_tf.translation.y;
}

pub fn drag_camera(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    mut camera: Query<&mut Transform, With<MainCamera>>,
    player: Query<(), With<IsLocalPlayer>>,
) {
    if !player.is_empty() {
        return;
    }
    if accumulated_mouse_motion.delta == Vec2::ZERO
        || !mouse_button_input.pressed(MouseButton::Left)
    {
        return;
    }
    let Ok(mut transform) = camera.single_mut() else {
        return;
    };
    transform.translation += (accumulated_mouse_motion.delta * Vec2::new(-1.0, 1.0)).extend(0.0);
}

pub fn clamp_camera(
    map_bounds: Option<Res<MapBounds>>,
    mut camera: Query<(&mut Transform, &Projection), With<MainCamera>>,
) {
    let Some(bounds) = map_bounds else { return };
    let Ok((mut transform, projection)) = camera.single_mut() else {
        return;
    };
    let Projection::Orthographic(projection) = projection else {
        return;
    };

    let half_w = projection.area.width() * 0.5;
    let half_h = projection.area.height() * 0.5;

    let mut min_x = bounds.left + half_w;
    let mut max_x = bounds.right - half_w;
    let mut min_y = bounds.bottom + half_h;
    let mut max_y = bounds.top - half_h;

    if min_x > max_x {
        let mid = (min_x + max_x) * 0.5;
        min_x = mid;
        max_x = mid;
    }
    if min_y > max_y {
        let mid = (min_y + max_y) * 0.5;
        min_y = mid;
        max_y = mid;
    }

    transform.translation.x = transform.translation.x.clamp(min_x, max_x);
    transform.translation.y = transform.translation.y.clamp(min_y, max_y);
}

pub fn apply_resolution(
    base: Res<BaseResolution>,
    window: Query<&Window>,
    mut ui_scale: ResMut<UiScale>,
) {
    let Some(window) = window.iter().next() else {
        return;
    };
    ui_scale.0 = window.height() / base.height;
}
