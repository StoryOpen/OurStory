use super::resources::{BaseResolution, MainCamera};
use crate::map::events::MapLoaded;
use crate::map::resources::MapBounds;
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::prelude::*;
use bevy::ui::UiScale;

const ZOOM_SPEED: f32 = 0.1;
const ZOOM_MIN: f32 = 0.1;
const ZOOM_MAX: f32 = 5.0;

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

pub fn drag_camera(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    mut camera: Query<&mut Transform, With<MainCamera>>,
) {
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

pub fn draw_camera_viewport(
    mut gizmos: Gizmos,
    base: Res<BaseResolution>,
    camera: Query<&Transform, With<MainCamera>>,
) {
    let Ok(transform) = camera.single() else {
        return;
    };
    let size = Vec2::new(base.width, base.height);
    gizmos.rect_2d(transform.translation.truncate(), size, Color::srgba(1.0, 0.0, 0.0, 0.5));
}

pub fn zoom_camera(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_wheel: Res<AccumulatedMouseScroll>,
    mut camera: Query<&mut Projection, With<MainCamera>>,
) {
    if !keyboard.pressed(KeyCode::ControlLeft) && !keyboard.pressed(KeyCode::ControlRight) {
        return;
    }
    let delta = mouse_wheel.delta.y;
    if delta == 0.0 {
        return;
    }
    let Ok(mut projection) = camera.single_mut() else {
        return;
    };
    let Projection::Orthographic(ref mut orthographic) = *projection else {
        return;
    };
    let delta_zoom = -delta * ZOOM_SPEED;
    let multiplicative_zoom = 1.0 + delta_zoom;
    orthographic.scale = (orthographic.scale * multiplicative_zoom).clamp(ZOOM_MIN, ZOOM_MAX);
}
