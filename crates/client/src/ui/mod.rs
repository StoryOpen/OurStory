pub mod components;
pub mod loader;
pub mod windows;

use bevy::prelude::*;

use components::UiButton;
use loader::WzImageCache;
use windows::{hud, stat};
use crate::GameSet;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WzImageCache>()
            .add_systems(Startup, setup_ui)
            .add_systems(Update, update_button_sprites.in_set(GameSet::Ui));
    }
}

fn setup_ui(
    mut commands: Commands,
    mut cache: ResMut<WzImageCache>,
    mut images: ResMut<Assets<Image>>,
) {
    hud::spawn_hud(&mut commands, &mut cache, &mut images);
    stat::spawn_stat_window(&mut commands, &mut cache, &mut images);
}

fn update_button_sprites(mut query: Query<(&Interaction, &mut UiButton, &mut ImageNode)>) {
    for (interaction, button, mut image_node) in query.iter_mut() {
        let new_image = match interaction {
            Interaction::None => button.normal.clone(),
            Interaction::Hovered => button.hover.clone(),
            Interaction::Pressed => button.pressed.clone(),
        };
        if image_node.image != new_image {
            image_node.image = new_image;
        }
    }
}
