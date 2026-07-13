use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::camera::resources::MainCamera;
use crate::GameSet;

/// Marker component for sprites that can be picked and moved.
#[derive(Component, Reflect, Default)]
pub struct Pickable;

/// Marker component for the currently selected sprite.
#[derive(Component, Reflect, Default)]
pub struct Selected;

/// Resource to track the currently selected entity.
#[derive(Resource, Default)]
pub struct PickingState {
    pub selected: Option<Entity>,
    pub dragging: bool,
    pub drag_offset: Vec2,
}

pub struct PickingPlugin;

impl Plugin for PickingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PickingState>()
            .register_type::<Pickable>()
            .register_type::<Selected>()
            .add_systems(
                Update,
                (
                    pick_sprite,
                    move_selected_with_keys,
                    drag_selected_with_mouse,
                    draw_selection_gizmo,
                )
                    .chain()
                    .in_set(GameSet::Input),
            );
    }
}

/// Convert screen coordinates to world coordinates using the camera.
fn screen_to_world(
    screen_pos: Vec2,
    camera_transform: &Transform,
    camera_projection: &Projection,
    window: &Window,
) -> Vec2 {
    let Projection::Orthographic(projection) = camera_projection else {
        return Vec2::ZERO;
    };

    let window_size = Vec2::new(window.width(), window.height());
    let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;
    let ndc = Vec2::new(ndc.x, -ndc.y);

    let world_pos = camera_transform.translation.truncate()
        + ndc * Vec2::new(projection.area.width(), projection.area.height()) * 0.5;

    world_pos
}

/// Pick a sprite on mouse click.
fn pick_sprite(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Transform, &Projection), With<MainCamera>>,
    sprites: Query<(Entity, &Transform, &Sprite), With<Pickable>>,
    images: Res<Assets<Image>>,
    mut picking_state: ResMut<PickingState>,
    mut commands: Commands,
) {
    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let Ok((camera_transform, camera_projection)) = camera.single() else {
        return;
    };

    let world_pos = screen_to_world(cursor_pos, camera_transform, camera_projection, window);

    // Clear previous selection
    if let Some(prev_selected) = picking_state.selected {
        commands.entity(prev_selected).remove::<Selected>();
        picking_state.selected = None;
    }

    // Check sprites
    for (entity, transform, sprite) in sprites.iter() {
        let sprite_pos = transform.translation.truncate();

        let sprite_size = if let Some(custom_size) = sprite.custom_size {
            custom_size
        } else if let Some(image) = images.get(&sprite.image) {
            Vec2::new(image.width() as f32, image.height() as f32)
        } else {
            Vec2::new(100.0, 100.0)
        };

        let half_size = sprite_size * 0.5;
        let min = sprite_pos - half_size;
        let max = sprite_pos + half_size;

        if world_pos.x >= min.x
            && world_pos.x <= max.x
            && world_pos.y >= min.y
            && world_pos.y <= max.y
        {
            commands.entity(entity).insert(Selected);
            picking_state.selected = Some(entity);
            picking_state.dragging = false;
            picking_state.drag_offset = sprite_pos - world_pos;
            return;
        }
    }
}

/// Move selected sprite with arrow keys (1 pixel at a time).
fn move_selected_with_keys(
    keyboard: Res<ButtonInput<KeyCode>>,
    picking_state: Res<PickingState>,
    mut sprites: Query<&mut Transform, With<Selected>>,
) {
    let Some(selected) = picking_state.selected else {
        return;
    };

    let Ok(mut transform) = sprites.get_mut(selected) else {
        return;
    };

    let speed = 1.0;

    if keyboard.just_released(KeyCode::ArrowLeft) {
        transform.translation.x -= speed;
    }
    if keyboard.just_released(KeyCode::ArrowRight) {
        transform.translation.x += speed;
    }
    if keyboard.just_released(KeyCode::ArrowUp) {
        transform.translation.y += speed;
    }
    if keyboard.just_released(KeyCode::ArrowDown) {
        transform.translation.y -= speed;
    }
}

/// Drag selected sprite with mouse.
fn drag_selected_with_mouse(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Transform, &Projection), With<MainCamera>>,
    mut picking_state: ResMut<PickingState>,
    mut sprites: Query<&mut Transform, (With<Selected>, Without<MainCamera>)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let Ok((camera_transform, camera_projection)) = camera.single() else {
        return;
    };

    if mouse_button.just_pressed(MouseButton::Left) {
        if picking_state.selected.is_some() {
            picking_state.dragging = true;
        }
    }

    if mouse_button.just_released(MouseButton::Left) {
        picking_state.dragging = false;
    }

    if !picking_state.dragging {
        return;
    }

    let Some(selected) = picking_state.selected else {
        return;
    };

    let Ok(mut transform) = sprites.get_mut(selected) else {
        return;
    };

    let world_pos = screen_to_world(cursor_pos, camera_transform, camera_projection, window);
    transform.translation = (world_pos + picking_state.drag_offset).extend(transform.translation.z);
}

/// Draw a gizmo border around the selected sprite.
fn draw_selection_gizmo(
    picking_state: Res<PickingState>,
    sprites: Query<(&Transform, &Sprite), With<Selected>>,
    images: Res<Assets<Image>>,
    mut gizmos: Gizmos,
) {
    let Some(selected) = picking_state.selected else {
        return;
    };

    let Ok((transform, sprite)) = sprites.get(selected) else {
        return;
    };

    let sprite_size = if let Some(custom_size) = sprite.custom_size {
        custom_size
    } else if let Some(image) = images.get(&sprite.image) {
        Vec2::new(image.width() as f32, image.height() as f32)
    } else {
        Vec2::new(100.0, 100.0)
    };

    let pos = transform.translation.truncate();
    gizmos.rect_2d(pos, sprite_size, Color::srgb(0.0, 1.0, 0.0));
}
